//! Núcleo de ROBLOX VAULT: SQLite para metadatos, archivos locales
//! direccionados por contenido (SHA-256) para los originales.
//! Independiente de Tauri para poder probarlo y reutilizarlo desde el MCP.

pub mod roblox;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
}
pub type Result<T> = std::result::Result<T, VaultError>;

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/001_init.sql")), (2, include_str!("../migrations/002_library.sql")), (3, include_str!("../migrations/003_memory.sql"))];
pub const STATUSES: &[&str] = &["VERIFIED", "PARTIALLY VERIFIED", "PENDING", "FAILED", "DEPRECATED"];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project { pub id: String, pub name: String, pub description: String, pub architecture: String, pub created_at: String, pub updated_at: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Asset {
    pub id: String, pub project_id: Option<String>, pub project_name: Option<String>, pub kind: String,
    pub name: String, pub format: String, pub original_name: String, pub status: String,
    pub metadata: serde_json::Value, pub version: i64, pub sha256: String, pub size: i64,
    pub created_at: String, pub updated_at: String,
}

#[derive(Serialize, Debug)]
pub struct ImportResult { pub asset: Asset, pub duplicate_of: Option<String>, pub new_version: bool }

#[derive(Serialize, Debug, Default)]
pub struct Stats { pub projects: i64, pub assets: i64, pub by_kind: Vec<(String, i64)>, pub by_status: Vec<(String, i64)>, pub storage_bytes: i64, pub activity_24h: i64 }

#[derive(Serialize, Debug)]
pub struct Activity { pub id: i64, pub at: String, pub actor: String, pub action: String, pub target: String, pub detail: String, pub destructive: bool }

#[derive(Serialize, Debug)]
pub struct IntegrityReport { pub checked: i64, pub ok: i64, pub missing: Vec<String>, pub corrupted: Vec<String> }

pub struct Vault { pub root: PathBuf, pub conn: Connection }

fn now() -> String { Utc::now().to_rfc3339() }

pub fn sha256_file(p: &Path) -> Result<(String, u64)> {
    let mut f = fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut n = 0u64;
    loop { let r = f.read(&mut buf)?; if r == 0 { break; } h.update(&buf[..r]); n += r as u64; }
    Ok((hex::encode(h.finalize()), n))
}

/// Clasifica por extensión. Devuelve (kind, format).
pub fn classify(name: &str) -> (String, String) {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let kind = match ext.as_str() {
        "glb" | "gltf" | "fbx" | "obj" | "rbxm" | "rbxmx" => "model",
        "lua" | "luau" => "script",
        "png" | "jpg" | "jpeg" | "tga" | "bmp" => "texture",
        "json" | "toml" | "yaml" | "yml" => "config",
        "ogg" | "mp3" | "wav" => "audio",
        _ => "other",
    };
    (kind.into(), if ext.is_empty() { "bin".into() } else { ext })
}

/// Rechaza rutas relativas con `..`, absolutas o con prefijo (path traversal).
pub fn safe_relative(p: &str) -> Result<PathBuf> {
    let path = Path::new(p);
    for c in path.components() {
        match c { Component::Normal(_) | Component::CurDir => {}, _ => return Err(VaultError::Invalid(format!("ruta insegura: {p}"))) }
    }
    if p.is_empty() { return Err(VaultError::Invalid("ruta vacía".into())); }
    Ok(path.to_path_buf())
}

impl Vault {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        for d in ["blobs", "backups", "restored", "thumbs"] { fs::create_dir_all(root.join(d))?; }
        let conn = Connection::open(root.join("vault.db"))?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        let v = Vault { root, conn };
        v.migrate()?;
        Ok(v)
    }

    fn migrate(&self) -> Result<()> {
        let cur: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (ver, sql) in MIGRATIONS {
            if *ver > cur {
                self.conn.execute_batch(&format!("BEGIN; {sql} PRAGMA user_version={ver}; COMMIT;"))?;
            }
        }
        Ok(())
    }

    pub fn log(&self, actor: &str, action: &str, target: &str, detail: &str, destructive: bool) -> Result<()> {
        self.log_asset(actor, action, None, target, detail, destructive)
    }

    pub fn log_asset(&self, actor: &str, action: &str, asset_id: Option<&str>, target: &str, detail: &str, destructive: bool) -> Result<()> {
        self.conn.execute("INSERT INTO activity(at,actor,action,target,detail,destructive,asset_id) VALUES (?,?,?,?,?,?,?)",
            params![now(), actor, action, target, detail, destructive as i64, asset_id])?;
        Ok(())
    }

    // ---------- proyectos ----------
    pub fn create_project(&self, name: &str, description: &str) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() || name.len() > 120 { return Err(VaultError::Invalid("nombre de proyecto inválido".into())); }
        let p = Project { id: uuid::Uuid::new_v4().to_string(), name: name.into(), description: description.into(), architecture: String::new(), created_at: now(), updated_at: now() };
        self.conn.execute("INSERT INTO projects VALUES (?,?,?,?,?,?)", params![p.id, p.name, p.description, p.architecture, p.created_at, p.updated_at])
            .map_err(|e| if e.to_string().contains("UNIQUE") { VaultError::Invalid("ya existe un proyecto con ese nombre".into()) } else { e.into() })?;
        self.log("app", "project.create", &p.name, "", false)?;
        Ok(p)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut s = self.conn.prepare("SELECT id,name,description,architecture,created_at,updated_at FROM projects ORDER BY name")?;
        let rows = s.query_map([], |r| Ok(Project { id: r.get(0)?, name: r.get(1)?, description: r.get(2)?, architecture: r.get(3)?, created_at: r.get(4)?, updated_at: r.get(5)? }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ---------- almacenamiento ----------
    fn store_blob(&self, src: &Path) -> Result<(String, u64)> {
        let (hash, size) = sha256_file(src)?;
        let rel = format!("blobs/{}/{}", &hash[..2], hash);
        let dest = self.root.join(&rel);
        if !dest.exists() {
            fs::create_dir_all(dest.parent().unwrap())?;
            let tmp = dest.with_extension("tmp");
            fs::copy(src, &tmp)?;
            let (check, _) = sha256_file(&tmp)?;
            if check != hash { let _ = fs::remove_file(&tmp); return Err(VaultError::Invalid("la copia no coincide con el original".into())); }
            fs::rename(&tmp, &dest)?;
        }
        self.conn.execute("INSERT OR IGNORE INTO blobs VALUES (?,?,?,?)", params![hash, size as i64, rel, now()])?;
        if size <= 4 * 1024 * 1024 {
            if let Ok(text) = String::from_utf8(fs::read(&dest)?) {
                if !text.contains('\0') { self.conn.execute("INSERT OR IGNORE INTO blob_text VALUES (?,?)", params![hash, text])?; }
            }
        }
        Ok((hash, size))
    }

    /// Importa un archivo. El original del usuario nunca se modifica.
    /// - mismo hash ya existente → se informa duplicado (no se crea otra copia).
    /// - `update_asset_id` → se añade una versión nueva, sin sobrescribir la anterior.
    pub fn import_file(&self, src: &Path, project_id: Option<&str>, update_asset_id: Option<&str>) -> Result<ImportResult> {
        if !src.is_file() { return Err(VaultError::NotFound(src.display().to_string())); }
        let original = src.file_name().and_then(|n| n.to_str()).ok_or_else(|| VaultError::Invalid("nombre de archivo inválido".into()))?.to_string();
        let (hash, _) = sha256_file(src)?;
        let duplicate_of: Option<String> = self.conn.query_row(
            "SELECT asset_id FROM asset_versions WHERE sha256=? LIMIT 1", [&hash], |r| r.get(0)).optional()?;
        if let (Some(dup), None) = (&duplicate_of, update_asset_id) {
            return Ok(ImportResult { asset: self.get_asset(dup)?, duplicate_of: Some(dup.clone()), new_version: false });
        }
        self.store_blob(src)?;
        let t = now();
        let id = if let Some(aid) = update_asset_id {
            let a = self.get_asset(aid)?;
            if a.sha256 == hash { return Ok(ImportResult { asset: a, duplicate_of: Some(aid.into()), new_version: false }); }
            self.conn.execute("INSERT INTO asset_versions VALUES (?,?,?,?,?,?)", params![uuid::Uuid::new_v4().to_string(), aid, a.version + 1, hash, "", t])?;
            self.conn.execute("UPDATE assets SET updated_at=?, status='PENDING' WHERE id=?", params![t, aid])?;
            if let Some(extra) = roblox_meta(src, &a.format) {
                let mut m = a.metadata.clone();
                if let (Some(o), Some(e)) = (m.as_object_mut(), extra.as_object()) { for (k, v) in e { o.insert(k.clone(), v.clone()); } }
                self.conn.execute("UPDATE assets SET metadata=? WHERE id=?", params![m.to_string(), aid])?;
            }
            self.log_asset("app", "asset.version", Some(aid), &a.name, &format!("v{}", a.version + 1), false)?;
            aid.to_string()
        } else {
            let (kind, format) = classify(&original);
            let name = Path::new(&original).file_stem().and_then(|s| s.to_str()).unwrap_or(&original).to_string();
            let id = uuid::Uuid::new_v4().to_string();
            let (mut kind, format) = (kind, format);
            let (name, mut meta) = if kind == "script" { let (n, st) = script_type(&name); (n, serde_json::json!({ "script_type": st })) } else { (name, serde_json::json!({})) };
            if let Some(extra) = roblox_meta(src, &format) {
                let anims = extra["animations"].as_array().map_or(0, |a| a.len());
                if anims > 0 && extra["pieces"].as_u64() == Some(0) { kind = "animation".into(); }
                meta = extra;
            }
            self.conn.execute("INSERT INTO assets(id,project_id,kind,name,format,original_name,status,metadata,created_at,updated_at) VALUES (?,?,?,?,?,?, 'PENDING',?,?,?)",
                params![id, project_id, kind, name, format, original, meta.to_string(), t, t])?;
            self.conn.execute("INSERT INTO asset_versions VALUES (?,?,1,?,?,?)", params![uuid::Uuid::new_v4().to_string(), id, hash, "", t])?;
            self.log_asset("app", "asset.import", Some(&id), &original, &hash[..12], false)?;
            id
        };
        Ok(ImportResult { asset: self.get_asset(&id)?, duplicate_of, new_version: update_asset_id.is_some() })
    }

    const ASSET_SQL: &'static str = "SELECT a.id,a.project_id,p.name,a.kind,a.name,a.format,a.original_name,a.status,a.metadata,v.version,v.sha256,b.size,a.created_at,a.updated_at
        FROM assets a LEFT JOIN projects p ON p.id=a.project_id
        JOIN asset_versions v ON v.asset_id=a.id AND v.version=(SELECT MAX(version) FROM asset_versions WHERE asset_id=a.id)
        JOIN blobs b ON b.sha256=v.sha256";

    fn row_asset(r: &rusqlite::Row) -> rusqlite::Result<Asset> {
        let meta: String = r.get(8)?;
        Ok(Asset { id: r.get(0)?, project_id: r.get(1)?, project_name: r.get(2)?, kind: r.get(3)?, name: r.get(4)?, format: r.get(5)?,
            original_name: r.get(6)?, status: r.get(7)?, metadata: serde_json::from_str(&meta).unwrap_or_default(), version: r.get(9)?,
            sha256: r.get(10)?, size: r.get(11)?, created_at: r.get(12)?, updated_at: r.get(13)? })
    }

    pub fn get_asset(&self, id: &str) -> Result<Asset> {
        self.conn.query_row(&format!("{} WHERE a.id=?", Self::ASSET_SQL), [id], Self::row_asset).optional()?
            .ok_or_else(|| VaultError::NotFound(id.into()))
    }

    pub fn list_assets(&self, kind: Option<&str>, project_id: Option<&str>, query: Option<&str>) -> Result<Vec<Asset>> { self.search_assets(kind, project_id, query, true) }

    pub fn search_assets(&self, kind: Option<&str>, project_id: Option<&str>, query: Option<&str>, in_content: bool) -> Result<Vec<Asset>> {
        let q = query.map(|s| format!("%{}%", s.to_lowercase()));
        let mut s = self.conn.prepare(&format!("{} WHERE (?1 IS NULL OR a.kind=?1) AND (?2 IS NULL OR a.project_id=?2) AND (?3 IS NULL OR lower(a.name) LIKE ?3 OR lower(a.original_name) LIKE ?3 OR (?4 AND EXISTS (SELECT 1 FROM blob_text t WHERE t.sha256=v.sha256 AND lower(t.content) LIKE ?3))) ORDER BY a.updated_at DESC", Self::ASSET_SQL))?;
        let rows = s.query_map(params![kind, project_id, q, in_content], Self::row_asset)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn set_status(&self, id: &str, status: &str) -> Result<()> {
        if !STATUSES.contains(&status) { return Err(VaultError::Invalid(format!("estado inválido: {status}"))); }
        let n = self.conn.execute("UPDATE assets SET status=?, updated_at=? WHERE id=?", params![status, now(), id])?;
        if n == 0 { return Err(VaultError::NotFound(id.into())); }
        self.log_asset("app", "asset.status", Some(id), id, status, false)
    }

    /// Restaura una versión a `dest_dir/<nombre original>`. Nunca sobrescribe:
    /// si existe, añade sufijo numérico.
    pub fn restore_asset(&self, id: &str, version: Option<i64>, dest_dir: &Path) -> Result<PathBuf> {
        let a = self.get_asset(id)?;
        let v = version.unwrap_or(a.version);
        let (rel, hash): (String, String) = self.conn.query_row(
            "SELECT b.rel_path, b.sha256 FROM asset_versions v JOIN blobs b ON b.sha256=v.sha256 WHERE v.asset_id=? AND v.version=?",
            params![id, v], |r| Ok((r.get(0)?, r.get(1)?))).optional()?.ok_or_else(|| VaultError::NotFound(format!("{id} v{v}")))?;
        let src = self.root.join(safe_relative(&rel)?);
        if sha256_file(&src)?.0 != hash { return Err(VaultError::Invalid("el archivo almacenado está corrupto; restauración cancelada".into())); }
        fs::create_dir_all(dest_dir)?;
        let fname = safe_relative(&a.original_name)?;
        let stem = fname.file_stem().and_then(|s| s.to_str()).unwrap_or("asset").to_string();
        let ext = fname.extension().and_then(|s| s.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
        let mut dest = dest_dir.join(&fname);
        let mut i = 1;
        while dest.exists() { dest = dest_dir.join(format!("{stem} ({i}){ext}")); i += 1; }
        fs::copy(&src, &dest)?;
        self.log_asset("app", "asset.restore", Some(id), &a.name, &format!("v{v} -> {}", dest.display()), false)?;
        Ok(dest)
    }

    pub fn verify_integrity(&self) -> Result<IntegrityReport> {
        let mut s = self.conn.prepare("SELECT sha256, rel_path FROM blobs")?;
        let rows: Vec<(String, String)> = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?;
        let mut rep = IntegrityReport { checked: 0, ok: 0, missing: vec![], corrupted: vec![] };
        for (h, rel) in rows {
            rep.checked += 1;
            let p = self.root.join(safe_relative(&rel)?);
            if !p.exists() { rep.missing.push(h); continue; }
            if sha256_file(&p)?.0 == h { rep.ok += 1 } else { rep.corrupted.push(h) }
        }
        self.log("app", "integrity.check", "", &format!("{}/{} ok", rep.ok, rep.checked), false)?;
        Ok(rep)
    }

    /// Copia consistente de la base de datos (VACUUM INTO). Los blobs son inmutables.
    pub fn backup(&self) -> Result<PathBuf> {
        let dest = self.root.join("backups").join(format!("vault-{}.db", Utc::now().format("%Y%m%d-%H%M%S%.3f")));
        self.conn.execute("VACUUM INTO ?", [dest.to_string_lossy()])?;
        self.log("app", "backup.create", &dest.display().to_string(), "", false)?;
        Ok(dest)
    }

    pub fn stats(&self) -> Result<Stats> {
        let one = |sql: &str| -> Result<i64> { Ok(self.conn.query_row(sql, [], |r| r.get(0))?) };
        let pairs = |sql: &str| -> Result<Vec<(String, i64)>> {
            let mut s = self.conn.prepare(sql)?;
            let r = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?; Ok(r)
        };
        let since = (Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        Ok(Stats {
            projects: one("SELECT COUNT(*) FROM projects")?,
            assets: one("SELECT COUNT(*) FROM assets")?,
            by_kind: pairs("SELECT kind, COUNT(*) FROM assets GROUP BY kind ORDER BY 2 DESC")?,
            by_status: pairs("SELECT status, COUNT(*) FROM assets GROUP BY status ORDER BY 2 DESC")?,
            storage_bytes: one("SELECT COALESCE(SUM(size),0) FROM blobs")?,
            activity_24h: self.conn.query_row("SELECT COUNT(*) FROM activity WHERE at>=?", [since], |r| r.get(0))?,
        })
    }

    pub fn recent_activity(&self, limit: i64) -> Result<Vec<Activity>> {
        let mut s = self.conn.prepare("SELECT id,at,actor,action,target,detail,destructive FROM activity ORDER BY id DESC LIMIT ?")?;
        let r = s.query_map([limit.clamp(1, 500)], |r| Ok(Activity { id: r.get(0)?, at: r.get(1)?, actor: r.get(2)?, action: r.get(3)?, target: r.get(4)?, detail: r.get(5)?, destructive: r.get::<_, i64>(6)? != 0 }))?;
        Ok(r.collect::<std::result::Result<_, _>>()?)
    }
}



/// Convención Rojo: `X.server.luau` → Script, `X.client.luau` → LocalScript, resto → ModuleScript.
pub fn script_type(stem: &str) -> (String, &'static str) {
    if let Some(n) = stem.strip_suffix(".server") { (n.into(), "Script") }
    else if let Some(n) = stem.strip_suffix(".client") { (n.into(), "LocalScript") }
    else { (stem.into(), "ModuleScript") }
}

#[derive(Serialize, Debug)]
pub struct Version { pub version: i64, pub sha256: String, pub size: i64, pub note: String, pub created_at: String }

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SystemInput { pub name: String, pub project_id: Option<String>, pub description: String, pub install: String, pub config: String, pub compatibility: String, pub tests: String }

#[derive(Serialize, Debug)]
pub struct SystemMember { pub role: String, pub asset: Asset }

#[derive(Serialize, Debug)]
pub struct System {
    pub id: String, pub project_id: Option<String>, pub name: String, pub description: String, pub install: String, pub config: String,
    pub compatibility: String, pub tests: String, pub status: String, pub created_at: String, pub updated_at: String,
    pub members: Vec<SystemMember>, pub warnings: Vec<String>,
}

impl Vault {
    pub fn list_versions(&self, asset_id: &str) -> Result<Vec<Version>> {
        let mut s = self.conn.prepare("SELECT v.version,v.sha256,b.size,v.note,v.created_at FROM asset_versions v JOIN blobs b ON b.sha256=v.sha256 WHERE v.asset_id=? ORDER BY v.version DESC")?;
        let r = s.query_map([asset_id], |r| Ok(Version { version: r.get(0)?, sha256: r.get(1)?, size: r.get(2)?, note: r.get(3)?, created_at: r.get(4)? }))?;
        Ok(r.collect::<std::result::Result<_, _>>()?)
    }

    /// Texto de una versión (solo archivos de texto). `None` si es binario.
    pub fn get_text(&self, asset_id: &str, version: Option<i64>) -> Result<Option<String>> {
        let v = match version { Some(v) => v, None => self.get_asset(asset_id)?.version };
        Ok(self.conn.query_row("SELECT t.content FROM asset_versions v JOIN blob_text t ON t.sha256=v.sha256 WHERE v.asset_id=? AND v.version=?",
            params![asset_id, v], |r| r.get(0)).optional()?)
    }

    pub fn update_asset(&self, id: &str, name: Option<&str>, project_id: Option<Option<&str>>, metadata: Option<&serde_json::Value>) -> Result<Asset> {
        let a = self.get_asset(id)?;
        let name = name.map(str::trim).unwrap_or(&a.name);
        if name.is_empty() { return Err(VaultError::Invalid("nombre vacío".into())); }
        let pid = match project_id { Some(p) => p.map(String::from), None => a.project_id.clone() };
        let meta = metadata.cloned().unwrap_or(a.metadata.clone());
        if !meta.is_object() { return Err(VaultError::Invalid("metadata debe ser un objeto".into())); }
        self.conn.execute("UPDATE assets SET name=?, project_id=?, metadata=?, updated_at=? WHERE id=?", params![name, pid, meta.to_string(), now(), id])?;
        self.log_asset("app", "asset.update", Some(id), name, "", false)?;
        self.get_asset(id)
    }

    pub fn create_system(&self, i: &SystemInput) -> Result<System> {
        if i.name.trim().is_empty() { return Err(VaultError::Invalid("nombre de sistema vacío".into())); }
        let id = uuid::Uuid::new_v4().to_string(); let t = now();
        self.conn.execute("INSERT INTO systems(id,project_id,name,description,install,config,compatibility,tests,status,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,'PENDING',?,?)",
            params![id, i.project_id, i.name.trim(), i.description, i.install, i.config, i.compatibility, i.tests, t, t])?;
        self.log("app", "system.create", i.name.trim(), "", false)?;
        self.get_system(&id)
    }

    pub fn update_system(&self, id: &str, i: &SystemInput) -> Result<System> {
        if i.name.trim().is_empty() { return Err(VaultError::Invalid("nombre de sistema vacío".into())); }
        let n = self.conn.execute("UPDATE systems SET project_id=?,name=?,description=?,install=?,config=?,compatibility=?,tests=?,updated_at=? WHERE id=?",
            params![i.project_id, i.name.trim(), i.description, i.install, i.config, i.compatibility, i.tests, now(), id])?;
        if n == 0 { return Err(VaultError::NotFound(id.into())); }
        self.log("app", "system.update", i.name.trim(), "", false)?;
        self.get_system(id)
    }

    pub fn set_system_status(&self, id: &str, status: &str) -> Result<()> {
        if !STATUSES.contains(&status) { return Err(VaultError::Invalid(format!("estado inválido: {status}"))); }
        if self.conn.execute("UPDATE systems SET status=?, updated_at=? WHERE id=?", params![status, now(), id])? == 0 { return Err(VaultError::NotFound(id.into())); }
        self.log("app", "system.status", id, status, false)
    }

    pub fn add_system_member(&self, system_id: &str, asset_id: &str, role: &str) -> Result<()> {
        self.get_asset(asset_id)?;
        self.conn.execute("INSERT INTO system_members VALUES (?,?,?) ON CONFLICT(system_id,asset_id) DO UPDATE SET role=excluded.role", params![system_id, asset_id, role])
            .map_err(|e| if e.to_string().contains("FOREIGN KEY") { VaultError::NotFound(system_id.into()) } else { e.into() })?;
        self.log("app", "system.member.add", system_id, asset_id, false)
    }

    pub fn remove_system_member(&self, system_id: &str, asset_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM system_members WHERE system_id=? AND asset_id=?", params![system_id, asset_id])?;
        self.log("app", "system.member.remove", system_id, asset_id, true)
    }

    pub fn get_system(&self, id: &str) -> Result<System> {
        let mut sys = self.conn.query_row("SELECT id,project_id,name,description,install,config,compatibility,tests,status,created_at,updated_at FROM systems WHERE id=?", [id], |r| Ok(System {
            id: r.get(0)?, project_id: r.get(1)?, name: r.get(2)?, description: r.get(3)?, install: r.get(4)?, config: r.get(5)?, compatibility: r.get(6)?,
            tests: r.get(7)?, status: r.get(8)?, created_at: r.get(9)?, updated_at: r.get(10)?, members: vec![], warnings: vec![] })).optional()?
            .ok_or_else(|| VaultError::NotFound(id.into()))?;
        let mut s = self.conn.prepare("SELECT asset_id, role FROM system_members WHERE system_id=? ORDER BY role, asset_id")?;
        let rows: Vec<(String, String)> = s.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?;
        for (aid, role) in rows {
            let asset = self.get_asset(&aid)?;
            let blob = self.root.join("blobs").join(&asset.sha256[..2]).join(&asset.sha256);
            if !blob.exists() { sys.warnings.push(format!("{}: falta el archivo original", asset.name)); }
            match asset.status.as_str() {
                "FAILED" => sys.warnings.push(format!("{}: marcado como FAILED", asset.name)),
                "DEPRECATED" => sys.warnings.push(format!("{}: marcado como DEPRECATED", asset.name)),
                _ => {}
            }
            sys.members.push(SystemMember { role, asset });
        }
        if sys.members.is_empty() { sys.warnings.push("el sistema no tiene recursos".into()); }
        Ok(sys)
    }

    pub fn list_systems(&self, project_id: Option<&str>, query: Option<&str>) -> Result<Vec<System>> {
        let q = query.filter(|q| !q.is_empty()).map(|q| format!("%{}%", q.to_lowercase()));
        let mut s = self.conn.prepare("SELECT id FROM systems WHERE (?1 IS NULL OR project_id=?1) AND (?2 IS NULL OR lower(name) LIKE ?2 OR lower(description) LIKE ?2) ORDER BY name")?;
        let ids: Vec<String> = s.query_map(params![project_id, q], |r| r.get(0))?.collect::<std::result::Result<_, _>>()?;
        ids.iter().map(|i| self.get_system(i)).collect()
    }

    /// Restaura todos los recursos de un sistema en `dest_dir/<sistema>/`. Aborta si falta algún original.
    pub fn restore_system(&self, id: &str, dest_dir: &Path) -> Result<Vec<PathBuf>> {
        let sys = self.get_system(id)?;
        if let Some(w) = sys.warnings.iter().find(|w| w.contains("falta el archivo")) { return Err(VaultError::Invalid(format!("dependencia ausente: {w}"))); }
        let safe: String = sys.name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
        let dir = dest_dir.join(safe.trim());
        let out = sys.members.iter().map(|m| self.restore_asset(&m.asset.id, None, &dir)).collect::<Result<Vec<_>>>()?;
        self.log("app", "system.restore", &sys.name, &dir.display().to_string(), false)?;
        Ok(out)
    }
}


fn roblox_meta(src: &Path, format: &str) -> Option<serde_json::Value> {
    if format != "rbxm" && format != "rbxmx" { return None; }
    let bytes = fs::read(src).ok()?;
    Some(match roblox::parse(&bytes, format) {
        Ok(m) => roblox::summary(&m),
        Err(e) => serde_json::json!({ "parse_error": e }),
    })
}

impl Vault {
    fn blob_path(&self, asset_id: &str, version: Option<i64>) -> Result<(PathBuf, Asset)> {
        let a = self.get_asset(asset_id)?;
        let v = version.unwrap_or(a.version);
        let rel: String = self.conn.query_row("SELECT b.rel_path FROM asset_versions v JOIN blobs b ON b.sha256=v.sha256 WHERE v.asset_id=? AND v.version=?",
            params![asset_id, v], |r| r.get(0)).optional()?.ok_or_else(|| VaultError::NotFound(format!("{asset_id} v{v}")))?;
        Ok((self.root.join(safe_relative(&rel)?), a))
    }

    /// Bytes del original (para el visor 3D). Límite 512 MB.
    pub fn read_blob(&self, asset_id: &str, version: Option<i64>) -> Result<Vec<u8>> {
        let (p, _) = self.blob_path(asset_id, version)?;
        if fs::metadata(&p)?.len() > 512 * 1024 * 1024 { return Err(VaultError::Invalid("archivo demasiado grande para el visor".into())); }
        Ok(fs::read(p)?)
    }

    pub fn roblox_scene(&self, asset_id: &str, version: Option<i64>) -> Result<roblox::RbxModel> {
        let (p, a) = self.blob_path(asset_id, version)?;
        roblox::parse(&fs::read(p)?, &a.format).map_err(VaultError::Invalid)
    }

    fn thumb_path(&self, asset_id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(asset_id).map_err(|_| VaultError::Invalid("id inválido".into()))?;
        Ok(self.root.join("thumbs").join(format!("{asset_id}.png")))
    }

    /// Guarda la miniatura renderizada (PNG). Valida la firma PNG.
    pub fn save_thumbnail(&self, asset_id: &str, png: &[u8]) -> Result<()> {
        self.get_asset(asset_id)?;
        if png.len() < 8 || &png[..8] != b"\x89PNG\r\n\x1a\n" || png.len() > 4 * 1024 * 1024 { return Err(VaultError::Invalid("miniatura PNG inválida".into())); }
        let p = self.thumb_path(asset_id)?;
        fs::create_dir_all(p.parent().unwrap())?;
        fs::write(p, png)?;
        Ok(())
    }

    pub fn thumbnail(&self, asset_id: &str) -> Result<Option<Vec<u8>>> {
        let p = self.thumb_path(asset_id)?;
        Ok(if p.exists() { Some(fs::read(p)?) } else { None })
    }
}


/// Nombres de módulos usados en `require(...)` dentro de código Luau.
/// Reconoce `require(x.Name)`, `require(x:WaitForChild("Name"))`, `require(x["Name"])` y `require(script.Parent.Name)`.
pub fn luau_requires(src: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut rest = src;
    while let Some(i) = rest.find("require") {
        rest = &rest[i + 7..];
        let r = rest.trim_start();
        if !r.starts_with('(') { continue; }
        let mut depth = 0; let mut end = 0;
        for (j, c) in r.char_indices() { if c == '(' { depth += 1 } else if c == ')' { depth -= 1; if depth == 0 { end = j; break; } } }
        if end == 0 { continue; }
        let arg = r[1..end].trim();
        let name = if let Some(q) = arg.rfind(|c| c == '"' || c == '\'') {
            let before = &arg[..q];
            before.rfind(|c| c == '"' || c == '\'').map(|s| before[s + 1..].to_string())
        } else {
            arg.rsplit(|c: char| c == '.' || c == ':').next().map(|s| s.trim().to_string())
        };
        if let Some(n) = name { if !n.is_empty() && !n.starts_with(|c: char| c.is_ascii_digit()) && n.chars().all(|c| c.is_alphanumeric() || c == '_') && !out.contains(&n) { out.push(n); } }
    }
    out
}

#[derive(Serialize, Debug)]
pub struct DepNode { pub asset: Asset, pub note: String, pub depth: usize }

#[derive(Serialize, Debug)]
pub struct DepReport { pub direct: Vec<DepNode>, pub all: Vec<DepNode>, pub dependents: Vec<Asset>, pub problems: Vec<String>, pub unresolved_requires: Vec<String>, pub suggestions: Vec<Asset> }

#[derive(Serialize, Debug, Clone)]
pub struct Capture { pub id: String, pub source_path: String, pub project_id: Option<String>, pub tested: bool, pub test_passed: Option<bool>, pub test_output: String, pub note: String, pub actor: String, pub state: String, pub asset_id: Option<String>, pub created_at: String }

#[derive(Serialize, Debug, Clone)]
pub struct ProjectNote { pub id: i64, pub kind: String, pub text: String, pub created_at: String }

#[derive(Serialize, Debug)]
pub struct ProjectContext { pub project: Project, pub assets: Vec<Asset>, pub systems: Vec<System>, pub notes: Vec<ProjectNote>, pub recent: Vec<Activity> }

impl Vault {
    // ---------- dependencias ----------
    pub fn add_dependency(&self, asset_id: &str, depends_on: &str, note: &str) -> Result<()> {
        if asset_id == depends_on { return Err(VaultError::Invalid("un recurso no puede depender de sí mismo".into())); }
        self.get_asset(asset_id)?; self.get_asset(depends_on)?;
        self.conn.execute("INSERT INTO asset_deps VALUES (?,?,?) ON CONFLICT DO UPDATE SET note=excluded.note", params![asset_id, depends_on, note])?;
        self.log_asset("app", "dependency.add", Some(asset_id), asset_id, depends_on, false)
    }
    pub fn remove_dependency(&self, asset_id: &str, depends_on: &str) -> Result<()> {
        self.conn.execute("DELETE FROM asset_deps WHERE asset_id=? AND depends_on=?", params![asset_id, depends_on])?;
        self.log_asset("app", "dependency.remove", Some(asset_id), asset_id, depends_on, true)
    }

    /// Dependencias directas y transitivas, quién depende de este recurso, problemas
    /// (faltantes, FAILED/DEPRECATED, ciclos) y `require` del código sin resolver.
    pub fn dependency_report(&self, asset_id: &str) -> Result<DepReport> {
        let root = self.get_asset(asset_id)?;
        let deps_of = |id: &str| -> Result<Vec<(String, String)>> {
            let mut st = self.conn.prepare("SELECT depends_on, note FROM asset_deps WHERE asset_id=?")?;
            let r = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?; Ok(r)
        };
        let mut all: Vec<DepNode> = vec![]; let mut problems = vec![];
        let mut seen = std::collections::HashSet::from([asset_id.to_string()]);
        let mut stack: Vec<(String, String, usize, Vec<String>)> = deps_of(asset_id)?.into_iter().map(|(d, n)| (d, n, 1, vec![asset_id.to_string()])).collect();
        stack.reverse();
        while let Some((id, note, depth, chain)) = stack.pop() {
            if chain.contains(&id) { problems.push(format!("ciclo de dependencias: {} → {id}", chain.join(" → "))); continue; }
            if !seen.insert(id.clone()) { continue; }
            let a = self.get_asset(&id)?;
            let blob = self.root.join("blobs").join(&a.sha256[..2]).join(&a.sha256);
            if !blob.exists() { problems.push(format!("{}: falta el archivo original", a.name)); }
            if a.status == "FAILED" || a.status == "DEPRECATED" { problems.push(format!("{}: {}", a.name, a.status)); }
            let mut ch = chain.clone(); ch.push(id.clone());
            for (d, n) in deps_of(&id)?.into_iter().rev() { stack.push((d, n, depth + 1, ch.clone())); }
            all.push(DepNode { asset: a, note, depth });
        }
        let direct = deps_of(asset_id)?.into_iter().map(|(d, n)| Ok(DepNode { asset: self.get_asset(&d)?, note: n, depth: 1 })).collect::<Result<Vec<_>>>()?;
        let mut st = self.conn.prepare("SELECT asset_id FROM asset_deps WHERE depends_on=?")?;
        let dependents = st.query_map([asset_id], |r| r.get::<_, String>(0))?.collect::<std::result::Result<Vec<_>, _>>()?.iter().map(|i| self.get_asset(i)).collect::<Result<Vec<_>>>()?;
        // require() sin resolver y sugerencias por nombre
        let mut unresolved = vec![]; let mut suggestions = vec![];
        if let Some(text) = self.get_text(asset_id, None)? {
            for name in luau_requires(&text) {
                if direct.iter().any(|d| d.asset.name == name) { continue; }
                let found: Vec<Asset> = self.search_assets(Some("script"), None, None, false)?.into_iter().filter(|a| a.name == name && a.id != root.id).collect();
                if found.is_empty() { unresolved.push(name) } else { suggestions.extend(found) }
            }
        }
        Ok(DepReport { direct, all, dependents, problems, unresolved_requires: unresolved, suggestions })
    }

    pub fn asset_history(&self, asset_id: &str, limit: i64) -> Result<Vec<Activity>> {
        let mut s = self.conn.prepare("SELECT id,at,actor,action,target,detail,destructive FROM activity WHERE asset_id=? ORDER BY id DESC LIMIT ?")?;
        let r = s.query_map(params![asset_id, limit.clamp(1, 500)], |r| Ok(Activity { id: r.get(0)?, at: r.get(1)?, actor: r.get(2)?, action: r.get(3)?, target: r.get(4)?, detail: r.get(5)?, destructive: r.get::<_, i64>(6)? != 0 }))?;
        Ok(r.collect::<std::result::Result<_, _>>()?)
    }

    // ---------- ajustes ----------
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM settings WHERE key=?", [key], |r| r.get(0)).optional()?)
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute("INSERT INTO settings VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])?;
        self.log("app", "settings.set", key, value, false)
    }
    pub fn auto_capture_tested(&self) -> Result<bool> { Ok(self.setting("auto_capture_tested")?.as_deref() != Some("false")) }

    // ---------- captura híbrida ----------
    /// Registra un recurso construido (p. ej. por Claude):
    /// - probado y pasó + auto-captura activa → se importa ya como PARTIALLY VERIFIED;
    /// - probado y falló → se importa como FAILED (nunca como funcional);
    /// - no probado (o auto-captura apagada) → queda pendiente de aprobación.
    pub fn capture(&self, src: &Path, project_id: Option<&str>, tested: bool, test_passed: Option<bool>, test_output: &str, note: &str, actor: &str) -> Result<Capture> {
        if !src.is_file() { return Err(VaultError::NotFound(src.display().to_string())); }
        if tested && test_passed.is_none() { return Err(VaultError::Invalid("si tested=true hay que indicar test_passed".into())); }
        let id = uuid::Uuid::new_v4().to_string();
        let auto = tested && (test_passed == Some(false) || self.auto_capture_tested()?);
        self.conn.execute("INSERT INTO captures(id,source_path,project_id,tested,test_passed,test_output,note,actor,state,created_at) VALUES (?,?,?,?,?,?,?,?, 'pending', ?)",
            params![id, src.display().to_string(), project_id, tested, test_passed, test_output, note, actor, now()])?;
        self.log("app", "capture.create", &src.display().to_string(), if auto { "auto" } else { "pendiente de aprobación" }, false)?;
        if auto { self.decide_capture(&id, true, actor)?; }
        self.get_capture(&id)
    }

    pub fn get_capture(&self, id: &str) -> Result<Capture> {
        self.conn.query_row("SELECT id,source_path,project_id,tested,test_passed,test_output,note,actor,state,asset_id,created_at FROM captures WHERE id=?", [id], |r| Ok(Capture {
            id: r.get(0)?, source_path: r.get(1)?, project_id: r.get(2)?, tested: r.get(3)?, test_passed: r.get(4)?, test_output: r.get(5)?, note: r.get(6)?,
            actor: r.get(7)?, state: r.get(8)?, asset_id: r.get(9)?, created_at: r.get(10)? })).optional()?.ok_or_else(|| VaultError::NotFound(id.into()))
    }

    pub fn list_captures(&self, state: Option<&str>) -> Result<Vec<Capture>> {
        let mut s = self.conn.prepare("SELECT id FROM captures WHERE (?1 IS NULL OR state=?1) ORDER BY created_at DESC LIMIT 200")?;
        let ids: Vec<String> = s.query_map([state], |r| r.get(0))?.collect::<std::result::Result<_, _>>()?;
        ids.iter().map(|i| self.get_capture(i)).collect()
    }

    pub fn decide_capture(&self, id: &str, approve: bool, actor: &str) -> Result<Capture> {
        let c = self.get_capture(id)?;
        if c.state != "pending" { return Err(VaultError::Invalid("esta captura ya fue decidida".into())); }
        let mut asset_id = None;
        if approve {
            let r = self.import_file(Path::new(&c.source_path), c.project_id.as_deref(), None)?;
            let status = match (c.tested, c.test_passed) { (true, Some(true)) => "PARTIALLY VERIFIED", (true, Some(false)) => "FAILED", _ => "PENDING" };
            // un duplicado ya existente no se degrada a PENDING
            if r.duplicate_of.is_none() || status != "PENDING" { self.set_status(&r.asset.id, status)?; }
            if !c.test_output.is_empty() {
                let mut m = r.asset.metadata.clone();
                if let Some(o) = m.as_object_mut() {
                    o.insert("last_test".into(), serde_json::json!({ "passed": c.test_passed, "output": c.test_output.chars().take(4000).collect::<String>(), "at": now() }));
                }
                self.update_asset(&r.asset.id, None, None, Some(&m))?;
            }
            asset_id = Some(r.asset.id);
        }
        self.conn.execute("UPDATE captures SET state=?, asset_id=?, decided_at=? WHERE id=?", params![if approve { "approved" } else { "rejected" }, asset_id, now(), id])?;
        self.log("app", if approve { "capture.approve" } else { "capture.reject" }, &c.source_path, actor, !approve)?;
        self.get_capture(id)
    }

    // ---------- memoria de proyecto ----------
    pub fn add_project_note(&self, project_id: &str, kind: &str, text: &str) -> Result<ProjectNote> {
        if text.trim().is_empty() { return Err(VaultError::Invalid("nota vacía".into())); }
        self.conn.execute("INSERT INTO project_notes(project_id,kind,text,created_at) VALUES (?,?,?,?)", params![project_id, kind, text.trim(), now()])
            .map_err(|e| if e.to_string().contains("CHECK") { VaultError::Invalid(format!("tipo de nota inválido: {kind}")) } else if e.to_string().contains("FOREIGN") { VaultError::NotFound(project_id.into()) } else { e.into() })?;
        let id = self.conn.last_insert_rowid();
        self.log("app", "project.note", project_id, kind, false)?;
        Ok(ProjectNote { id, kind: kind.into(), text: text.trim().into(), created_at: now() })
    }
    pub fn delete_project_note(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM project_notes WHERE id=?", [id])?;
        self.log("app", "project.note.delete", &id.to_string(), "", true)
    }
    pub fn update_project(&self, id: &str, name: &str, description: &str, architecture: &str) -> Result<Project> {
        if name.trim().is_empty() { return Err(VaultError::Invalid("nombre vacío".into())); }
        if self.conn.execute("UPDATE projects SET name=?, description=?, architecture=?, updated_at=? WHERE id=?", params![name.trim(), description, architecture, now(), id])? == 0 { return Err(VaultError::NotFound(id.into())); }
        self.log("app", "project.update", name, "", false)?;
        self.list_projects()?.into_iter().find(|p| p.id == id).ok_or_else(|| VaultError::NotFound(id.into()))
    }
    pub fn project_context(&self, id: &str) -> Result<ProjectContext> {
        let project = self.list_projects()?.into_iter().find(|p| p.id == id).ok_or_else(|| VaultError::NotFound(id.into()))?;
        let mut s = self.conn.prepare("SELECT id,kind,text,created_at FROM project_notes WHERE project_id=? ORDER BY id DESC")?;
        let notes = s.query_map([id], |r| Ok(ProjectNote { id: r.get(0)?, kind: r.get(1)?, text: r.get(2)?, created_at: r.get(3)? }))?.collect::<std::result::Result<_, _>>()?;
        let mut s = self.conn.prepare("SELECT a.id,a.at,a.actor,a.action,a.target,a.detail,a.destructive FROM activity a JOIN assets x ON x.id=a.asset_id WHERE x.project_id=? ORDER BY a.id DESC LIMIT 30")?;
        let recent = s.query_map([id], |r| Ok(Activity { id: r.get(0)?, at: r.get(1)?, actor: r.get(2)?, action: r.get(3)?, target: r.get(4)?, detail: r.get(5)?, destructive: r.get::<_, i64>(6)? != 0 }))?.collect::<std::result::Result<_, _>>()?;
        Ok(ProjectContext { assets: self.list_assets(None, Some(id), None)?, systems: self.list_systems(Some(id), None)?, notes, recent, project })
    }
}


#[derive(Serialize, Debug, Clone)]
pub struct Check { pub name: String, pub ok: bool, pub detail: String }

#[derive(Serialize, Debug)]
pub struct ValidationReport { pub asset_id: String, pub name: String, pub checks: Vec<Check>, pub status_before: String, pub status_after: String, pub note: String }

/// Errores de sintaxis Luau (vacío = sin errores). Usa full_moon con el dialecto Luau.
pub fn luau_syntax_errors(code: &str) -> Vec<String> {
    full_moon::parse_fallible(code, full_moon::LuaVersion::luau()).errors().iter().take(20).map(|e| e.to_string()).collect()
}

impl Vault {
    /// Valida un recurso: integridad (SHA-256), estructura (Luau / RBXM / GLB / JSON), dependencias
    /// y, si se aportan, resultados reales de pruebas. Integridad ≠ funcionamiento:
    /// sin pruebas el máximo es PARTIALLY VERIFIED; con pruebas superadas → VERIFIED; cualquier fallo → FAILED.
    pub fn validate_asset(&self, asset_id: &str, test_passed: Option<bool>, test_output: &str, actor: &str) -> Result<ValidationReport> {
        let a = self.get_asset(asset_id)?;
        let mut checks = vec![];
        let (path, _) = self.blob_path(asset_id, None)?;
        let integrity = path.exists() && sha256_file(&path)?.0 == a.sha256;
        checks.push(Check { name: "integridad".into(), ok: integrity, detail: if integrity { "SHA-256 coincide".into() } else { "archivo ausente o modificado".into() } });
        if integrity {
            let bytes = fs::read(&path)?;
            let structure = match a.format.as_str() {
                "lua" | "luau" => match String::from_utf8(bytes.clone()) {
                    Ok(code) => { let e = luau_syntax_errors(&code); Some((e.is_empty(), if e.is_empty() { "sintaxis Luau válida".into() } else { e.join("; ") })) }
                    Err(_) => Some((false, "no es texto UTF-8".into())),
                },
                "rbxm" | "rbxmx" => Some(match roblox::parse(&bytes, &a.format) { Ok(m) => (true, format!("{} instancias, {} piezas, {} scripts", m.instance_count, m.parts.len(), m.scripts.len())), Err(e) => (false, e) }),
                "glb" => Some((bytes.len() >= 12 && &bytes[..4] == b"glTF", "cabecera glTF binaria".into())),
                "gltf" | "json" => Some(match serde_json::from_slice::<serde_json::Value>(&bytes) { Ok(_) => (true, "JSON válido".into()), Err(e) => (false, e.to_string()) }),
                _ => None,
            };
            if let Some((ok, detail)) = structure { checks.push(Check { name: "estructura".into(), ok, detail }); }
            // scripts dentro de modelos: sintaxis de cada uno
            if a.format == "rbxm" || a.format == "rbxmx" {
                if let Ok(m) = roblox::parse(&bytes, &a.format) {
                    let bad: Vec<String> = m.scripts.iter().filter_map(|s| { let e = luau_syntax_errors(&s.source); (!e.is_empty()).then(|| format!("{}: {}", s.path, e[0])) }).collect();
                    if !m.scripts.is_empty() { checks.push(Check { name: "scripts internos".into(), ok: bad.is_empty(), detail: if bad.is_empty() { format!("{} scripts con sintaxis válida", m.scripts.len()) } else { bad.join("; ") } }); }
                }
            }
        }
        let deps = self.dependency_report(asset_id)?;
        checks.push(Check { name: "dependencias".into(), ok: deps.problems.is_empty(), detail: if deps.problems.is_empty() { format!("{} dependencias correctas", deps.all.len()) } else { deps.problems.join("; ") } });
        if !deps.unresolved_requires.is_empty() {
            checks.push(Check { name: "require sin resolver".into(), ok: false, detail: format!("no están en la bóveda: {}", deps.unresolved_requires.join(", ")) });
        }
        if let Some(p) = test_passed { checks.push(Check { name: "pruebas".into(), ok: p, detail: test_output.chars().take(2000).collect() }); }

        // los require sin resolver no hacen fallar el recurso (pueden ser servicios de Roblox), solo impiden VERIFIED
        let hard_fail = checks.iter().any(|c| !c.ok && c.name != "require sin resolver");
        let soft_fail = checks.iter().any(|c| !c.ok);
        let (after, note) = if a.status == "DEPRECATED" { ("DEPRECATED", "el recurso está obsoleto; el estado no se cambia".to_string()) }
            else if hard_fail { ("FAILED", "al menos una comprobación falló".into()) }
            else if test_passed == Some(true) && !soft_fail { ("VERIFIED", "comprobaciones y pruebas reales superadas".into()) }
            else if a.status == "VERIFIED" && test_passed.is_none() { ("VERIFIED", "ya estaba verificado con pruebas; las comprobaciones siguen bien".into()) }
            else { ("PARTIALLY VERIFIED", if test_passed.is_none() { "estructura correcta, pero sin pruebas reales no se puede marcar como funcional".into() } else { "pruebas superadas, pero hay require sin resolver".into() }) };
        if after != a.status { self.conn.execute("UPDATE assets SET status=?, updated_at=? WHERE id=?", params![after, now(), asset_id])?; }
        let mut m = a.metadata.clone();
        if let Some(o) = m.as_object_mut() {
            o.insert("last_validation".into(), serde_json::json!({ "at": now(), "by": actor, "result": after, "checks": &checks }));
            if let Some(p) = test_passed { o.insert("last_test".into(), serde_json::json!({ "passed": p, "output": test_output.chars().take(4000).collect::<String>(), "at": now() })); }
        }
        self.conn.execute("UPDATE assets SET metadata=? WHERE id=?", params![m.to_string(), asset_id])?;
        self.log_asset(actor, "asset.validate", Some(asset_id), &a.name, after, false)?;
        Ok(ValidationReport { asset_id: asset_id.into(), name: a.name, checks, status_before: a.status, status_after: after.into(), note })
    }
}


/// Tokens para búsqueda: separa por símbolos y por camelCase/PascalCase ("InventoryUIController" → inventory, ui, controller).
pub fn search_tokens(s: &str) -> Vec<String> {
    let mut out = vec![];
    for word in s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let chars: Vec<char> = word.chars().collect();
        let mut cur = String::new();
        for (i, &c) in chars.iter().enumerate() {
            let boundary = i > 0 && ((c.is_uppercase() && chars[i - 1].is_lowercase())
                || (c.is_uppercase() && chars.get(i + 1).map_or(false, |n| n.is_lowercase()) && chars[i - 1].is_uppercase())
                || (c.is_ascii_digit() != chars[i - 1].is_ascii_digit()));
            if boundary && !cur.is_empty() { out.push(cur.to_lowercase()); cur.clear(); }
            cur.push(c);
        }
        if !cur.is_empty() { out.push(cur.to_lowercase()); }
        if word.len() > 1 { let w = word.to_lowercase(); if !out.contains(&w) { out.push(w); } }
    }
    out
}

#[derive(Serialize, Debug)]
pub struct Ranked { pub asset: Asset, pub score: f64, pub matched: Vec<String> }

impl Vault {
    /// Búsqueda por relevancia 100 % local (BM25 sobre nombre ×3, metadatos y código, con prefijos y camelCase).
    /// No usa servicios externos ni modelos de IA.
    pub fn search_ranked(&self, kind: Option<&str>, project_id: Option<&str>, query: &str, limit: usize) -> Result<Vec<Ranked>> {
        let q: Vec<String> = search_tokens(query).into_iter().filter(|t| t.len() >= 2).collect();
        if q.is_empty() { return Ok(self.search_assets(kind, project_id, None, false)?.into_iter().take(limit).map(|a| Ranked { asset: a, score: 0.0, matched: vec![] }).collect()); }
        let assets = self.search_assets(kind, project_id, None, false)?;
        let docs: Vec<(Asset, Vec<String>)> = assets.into_iter().map(|a| {
            let mut toks = vec![];
            for _ in 0..3 { toks.extend(search_tokens(&a.name)); }
            toks.extend(search_tokens(&a.original_name));
            toks.extend(search_tokens(&a.metadata.to_string()));
            if let Ok(Some(t)) = self.conn.query_row("SELECT content FROM blob_text WHERE sha256=?", [&a.sha256], |r| r.get::<_, String>(0)).optional() {
                toks.extend(search_tokens(&t.chars().take(200_000).collect::<String>()));
            }
            (a, toks)
        }).collect();
        let n = docs.len().max(1) as f64;
        let avg = docs.iter().map(|d| d.1.len()).sum::<usize>() as f64 / n;
        let substr = query.trim().to_lowercase();
        let mut out: Vec<Ranked> = docs.into_iter().filter_map(|(a, toks)| {
            let len = toks.len() as f64;
            let mut score = 0.0; let mut matched = vec![];
            for qt in &q {
                let (mut tf, mut exact) = (0.0, false);
                for t in &toks { if t == qt { tf += 1.0; exact = true } else if qt.len() >= 3 && t.starts_with(qt.as_str()) { tf += 0.6 } }
                if tf == 0.0 { continue; }
                // idf aproximado: cuantos más documentos lo contienen, menos peso (se recalcula abajo)
                let k1 = 1.2; let b = 0.75;
                score += tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * len / avg.max(1.0)));
                matched.push(if exact { qt.clone() } else { format!("{qt}*") });
            }
            if a.name.to_lowercase().contains(&substr) { score += 3.0 }
            if matched.len() == q.len() && q.len() > 1 { score *= 1.5 } // todas las palabras presentes
            (score > 0.0).then(|| Ranked { asset: a, score, matched })
        }).collect();
        // idf: penaliza términos muy comunes
        for qt in &q {
            let df = out.iter().filter(|r| r.matched.iter().any(|m| m.trim_end_matches('*') == qt)).count() as f64;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for r in out.iter_mut().filter(|r| r.matched.iter().any(|m| m.trim_end_matches('*') == qt)) { r.score *= 0.5 + idf / 2.0; }
        }
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(limit);
        for r in out.iter_mut() { r.score = (r.score * 100.0).round() / 100.0; }
        Ok(out)
    }
}


#[derive(Serialize, Debug, Default)]
pub struct FolderImport { pub imported: usize, pub duplicates: usize, pub skipped: usize, pub errors: Vec<String> }

/// Variante de icono por sufijo habitual en packs de Roblox ("CoinBlackStroke" → ("Coin", "BlackStroke")).
pub fn icon_variant(stem: &str) -> (String, Option<String>) {
    for v in ["BlackStroke", "WhiteStroke", "NoStroke"] {
        if stem.len() > v.len() && stem.to_lowercase().ends_with(&v.to_lowercase()) { return (stem[..stem.len() - v.len()].to_string(), Some(v.to_string())); }
    }
    (stem.to_string(), None)
}

impl Vault {
    /// Importa una carpeta entera (recursiva) como un pack: guarda pack, categoría (subcarpeta),
    /// variante y licencia en los metadatos. Solo tipos conocidos; nunca toca los originales.
    pub fn import_folder(&self, dir: &Path, pack: &str, project_id: Option<&str>) -> Result<FolderImport> {
        if !dir.is_dir() { return Err(VaultError::NotFound(dir.display().to_string())); }
        let mut license = None;
        let mut files = vec![];
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d)? {
                let p = e?.path();
                if p.is_dir() { stack.push(p); continue; }
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                if name.starts_with("license") || name.starts_with("licence") { license = fs::read_to_string(&p).ok(); }
                files.push(p);
            }
        }
        files.sort();
        let mut rep = FolderImport::default();
        for f in files {
            let fname = f.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let (kind, _) = classify(&fname);
            if kind == "other" || fname.to_lowercase().starts_with("license") { rep.skipped += 1; continue; }
            match self.import_file(&f, project_id, None) {
                Ok(r) if r.duplicate_of.is_some() => rep.duplicates += 1,
                Ok(r) => {
                    let rel = f.strip_prefix(dir).unwrap_or(&f);
                    let parts: Vec<String> = rel.parent().map(|p| p.components().filter_map(|c| c.as_os_str().to_str().map(String::from)).collect()).unwrap_or_default();
                    let stem = Path::new(&fname).file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let (base, variant) = icon_variant(stem);
                    let mut m = r.asset.metadata.clone();
                    if let Some(o) = m.as_object_mut() {
                        o.insert("pack".into(), serde_json::json!(pack));
                        o.insert("category".into(), serde_json::json!(parts.last().cloned().unwrap_or_default()));
                        o.insert("pack_path".into(), serde_json::json!(rel.to_string_lossy().replace('\\', "/")));
                        o.insert("icon".into(), serde_json::json!(base));
                        if let Some(v) = variant { o.insert("variant".into(), serde_json::json!(v)); }
                        if let Some(l) = &license { o.insert("license".into(), serde_json::json!(l.chars().take(2000).collect::<String>())); }
                    }
                    self.update_asset(&r.asset.id, None, None, Some(&m))?;
                    rep.imported += 1;
                }
                Err(e) => rep.errors.push(format!("{fname}: {e}")),
            }
        }
        self.log("app", "folder.import", pack, &format!("{} nuevos, {} duplicados", rep.imported, rep.duplicates), false)?;
        Ok(rep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(dir: &Path, name: &str, data: &[u8]) -> PathBuf { let p = dir.join(name); fs::write(&p, data).unwrap(); p }

    #[test]
    fn import_dedupe_version_restore_integrity() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        let p = v.create_project("Rift Tycoon", "test").unwrap();
        assert!(v.create_project("Rift Tycoon", "").is_err());

        let f = file(src.path(), "Inventory.luau", b"return {}");
        let r = v.import_file(&f, Some(&p.id), None).unwrap();
        assert_eq!(r.asset.kind, "script");
        assert_eq!(r.asset.version, 1);
        assert_eq!(r.asset.status, "PENDING");

        let dup = v.import_file(&f, None, None).unwrap();
        assert_eq!(dup.duplicate_of.as_deref(), Some(r.asset.id.as_str()));
        assert_eq!(v.list_assets(None, None, None).unwrap().len(), 1);

        fs::write(&f, b"return {v=2}").unwrap();
        let r2 = v.import_file(&f, None, Some(&r.asset.id)).unwrap();
        assert_eq!(r2.asset.version, 2);

        let out = tempfile::tempdir().unwrap();
        let a = v.restore_asset(&r.asset.id, Some(1), out.path()).unwrap();
        let b = v.restore_asset(&r.asset.id, Some(1), out.path()).unwrap();
        assert_ne!(a, b, "no debe sobrescribir");
        assert_eq!(fs::read(&a).unwrap(), b"return {}");

        let rep = v.verify_integrity().unwrap();
        assert_eq!((rep.checked, rep.ok), (2, 2));
        assert!(v.set_status(&r.asset.id, "WORKS").is_err());
        v.set_status(&r.asset.id, "VERIFIED").unwrap();

        let s = v.stats().unwrap();
        assert_eq!((s.projects, s.assets), (1, 1));
        assert!(v.backup().unwrap().exists());
        assert!(!v.recent_activity(10).unwrap().is_empty());
    }

    #[test]
    fn detects_corruption_and_traversal() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        let r = v.import_file(&file(src.path(), "m.rbxmx", b"<roblox/>"), None, None).unwrap();
        let blob = root.path().join("blobs").join(&r.asset.sha256[..2]).join(&r.asset.sha256);
        fs::write(&blob, b"tampered").unwrap();
        assert_eq!(v.verify_integrity().unwrap().corrupted.len(), 1);
        assert!(v.restore_asset(&r.asset.id, None, src.path()).is_err());
        assert!(safe_relative("../etc").is_err());
        assert!(safe_relative("C:\\x").is_ok() || cfg!(windows));
    }

    #[test]
    fn reopen_keeps_data() {
        let root = tempfile::tempdir().unwrap();
        { Vault::open(root.path()).unwrap().create_project("A", "").unwrap(); }
        assert_eq!(Vault::open(root.path()).unwrap().list_projects().unwrap().len(), 1);
    }

    #[test]
    fn systems_text_and_versions() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        let a = v.import_file(&file(src.path(), "InventoryController.client.luau", b"local Remote = game:GetService('ReplicatedStorage')"), None, None).unwrap().asset;
        assert_eq!(a.name, "InventoryController");
        assert_eq!(a.metadata["script_type"], "LocalScript");
        let b = v.import_file(&file(src.path(), "ItemConfig.luau", b"return { Sword = 1 }"), None, None).unwrap().asset;
        assert_eq!(v.list_assets(None, None, Some("replicatedstorage")).unwrap().len(), 1, "búsqueda por contenido");
        assert_eq!(v.get_text(&b.id, None).unwrap().as_deref(), Some("return { Sword = 1 }"));
        fs::write(src.path().join("ItemConfig.luau"), b"return { Sword = 2 }").unwrap();
        v.import_file(&src.path().join("ItemConfig.luau"), None, Some(&b.id)).unwrap();
        let vs = v.list_versions(&b.id).unwrap();
        assert_eq!(vs.len(), 2);
        assert_eq!(v.get_text(&b.id, Some(1)).unwrap().as_deref(), Some("return { Sword = 1 }"));

        let sys = v.create_system(&SystemInput { name: "Inventory System".into(), ..Default::default() }).unwrap();
        assert!(sys.warnings.iter().any(|w| w.contains("no tiene")));
        v.add_system_member(&sys.id, &a.id, "controller").unwrap();
        v.add_system_member(&sys.id, &b.id, "config").unwrap();
        v.set_status(&b.id, "FAILED").unwrap();
        let sys = v.get_system(&sys.id).unwrap();
        assert_eq!(sys.members.len(), 2);
        assert!(sys.warnings.iter().any(|w| w.contains("FAILED")));
        let out = tempfile::tempdir().unwrap();
        assert_eq!(v.restore_system(&sys.id, out.path()).unwrap().len(), 2);
        let blob = root.path().join("blobs").join(&a.sha256[..2]).join(&a.sha256);
        fs::remove_file(blob).unwrap();
        assert!(v.restore_system(&sys.id, out.path()).is_err(), "debe abortar si falta una dependencia");
        let up = v.update_asset(&a.id, Some("InvCtrl"), None, Some(&serde_json::json!({"script_type":"LocalScript","notes":"x"}))).unwrap();
        assert_eq!(up.name, "InvCtrl");
        assert!(v.update_asset(&a.id, None, None, Some(&serde_json::json!([1]))).is_err());
    }

    #[test]
    fn rbxmx_import_extracts_metadata_and_thumbnail() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        let xml = r#"<roblox version="4"><Item class="Part" referent="A"><Properties><string name="Name">P</string><Vector3 name="size"><X>2</X><Y>3</Y><Z>4</Z></Vector3></Properties></Item></roblox>"#;
        let a = v.import_file(&file(src.path(), "Crate.rbxmx", xml.as_bytes()), None, None).unwrap().asset;
        assert_eq!(a.metadata["pieces"], 1);
        assert_eq!(v.roblox_scene(&a.id, None).unwrap().parts[0].size, [2.0, 3.0, 4.0]);
        assert_eq!(v.read_blob(&a.id, None).unwrap(), xml.as_bytes());
        assert!(v.save_thumbnail(&a.id, b"notapng").is_err());
        v.save_thumbnail(&a.id, b"\x89PNG\r\n\x1a\nxxxx").unwrap();
        assert!(v.thumbnail(&a.id).unwrap().is_some());
        assert!(v.thumbnail("../../etc").is_err());
        let bad = v.import_file(&file(src.path(), "Bad.rbxm", b"garbage"), None, None).unwrap().asset;
        assert!(bad.metadata.get("parse_error").is_some());
    }

    #[test]
    fn deps_capture_and_project_memory() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        assert_eq!(luau_requires(r#"local A = require(game.ReplicatedStorage.Shared.ItemConfig)
            local B = require(script.Parent:WaitForChild("InventoryData")) local C=require(x["Util"]) require(123)"#), vec!["ItemConfig", "InventoryData", "Util"]);
        let ctrl = v.import_file(&file(src.path(), "Inv.luau", b"local C = require(script.Parent.ItemConfig)\nlocal D = require(script.Parent.Missing)"), None, None).unwrap().asset;
        let cfg = v.import_file(&file(src.path(), "ItemConfig.luau", b"return {}"), None, None).unwrap().asset;
        let rep = v.dependency_report(&ctrl.id).unwrap();
        assert_eq!(rep.suggestions[0].id, cfg.id);
        assert_eq!(rep.unresolved_requires, vec!["Missing"]);
        v.add_dependency(&ctrl.id, &cfg.id, "require").unwrap();
        assert!(v.add_dependency(&ctrl.id, &ctrl.id, "").is_err());
        let util = v.import_file(&file(src.path(), "Util.luau", b"return 1"), None, None).unwrap().asset;
        v.add_dependency(&cfg.id, &util.id, "").unwrap();
        v.set_status(&util.id, "FAILED").unwrap();
        let rep = v.dependency_report(&ctrl.id).unwrap();
        assert_eq!((rep.direct.len(), rep.all.len()), (1, 2));
        assert!(rep.problems.iter().any(|p| p.contains("FAILED")));
        assert_eq!(v.dependency_report(&cfg.id).unwrap().dependents[0].id, ctrl.id);
        v.add_dependency(&util.id, &ctrl.id, "").unwrap();
        assert!(v.dependency_report(&ctrl.id).unwrap().problems.iter().any(|p| p.contains("ciclo")));
        assert!(v.asset_history(&ctrl.id, 10).unwrap().iter().any(|a| a.action == "asset.import"));

        // captura híbrida
        let ok = v.capture(&file(src.path(), "Good.luau", b"return 'ok'"), None, true, Some(true), "3 passed", "", "claude").unwrap();
        assert_eq!(ok.state, "approved");
        let good = v.get_asset(ok.asset_id.as_ref().unwrap()).unwrap();
        assert_eq!(good.status, "PARTIALLY VERIFIED");
        assert_eq!(good.metadata["last_test"]["passed"], true);
        let bad = v.capture(&file(src.path(), "Bad.luau", b"error()"), None, true, Some(false), "1 failed", "", "claude").unwrap();
        assert_eq!(v.get_asset(bad.asset_id.as_ref().unwrap()).unwrap().status, "FAILED");
        let pend = v.capture(&file(src.path(), "New.luau", b"return 2"), None, false, None, "", "", "claude").unwrap();
        assert_eq!(pend.state, "pending");
        assert!(pend.asset_id.is_none());
        assert_eq!(v.list_captures(Some("pending")).unwrap().len(), 1);
        let ap = v.decide_capture(&pend.id, true, "user").unwrap();
        assert_eq!(v.get_asset(ap.asset_id.as_ref().unwrap()).unwrap().status, "PENDING");
        assert!(v.decide_capture(&pend.id, false, "user").is_err());
        v.set_setting("auto_capture_tested", "false").unwrap();
        assert_eq!(v.capture(&file(src.path(), "T.luau", b"return 3"), None, true, Some(true), "", "", "claude").unwrap().state, "pending");
        assert!(v.capture(&file(src.path(), "U.luau", b"return 4"), None, true, None, "", "", "claude").is_err());

        // memoria de proyecto
        let p = v.create_project("Rift", "").unwrap();
        v.add_project_note(&p.id, "decision", "Usar ProfileService").unwrap();
        assert!(v.add_project_note(&p.id, "random", "x").is_err());
        v.update_asset(&ctrl.id, None, Some(Some(&p.id)), None).unwrap();
        v.update_project(&p.id, "Rift Tycoon", "desc", "client/server").unwrap();
        let ctx = v.project_context(&p.id).unwrap();
        assert_eq!((ctx.project.architecture.as_str(), ctx.notes.len(), ctx.assets.len()), ("client/server", 1, 1));
        assert!(!ctx.recent.is_empty());
    }

    #[test]
    fn animation_import_kind() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        let xml = r#"<roblox version="4"><Item class="KeyframeSequence" referent="A"><Properties><string name="Name">Run</string></Properties><Item class="Keyframe" referent="B"><Properties><float name="Time">0.8</float></Properties></Item></Item></roblox>"#;
        let a = v.import_file(&file(src.path(), "Run.rbxmx", xml.as_bytes()), None, None).unwrap().asset;
        assert_eq!(a.kind, "animation");
        assert_eq!(a.metadata["animations"][0]["keyframes"], 1);
    }

    #[test]
    fn validation_rules() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        assert!(luau_syntax_errors("local x: number = if a then 1 else 2\nx += 1").is_empty(), "sintaxis Luau moderna");
        assert!(!luau_syntax_errors("local x = ").is_empty());
        let good = v.import_file(&file(src.path(), "Good.luau", b"local M = {}\nfunction M.f(a: number): number return a * 2 end\nreturn M"), None, None).unwrap().asset;
        let r = v.validate_asset(&good.id, None, "", "claude").unwrap();
        assert_eq!(r.status_after, "PARTIALLY VERIFIED", "sin pruebas no puede ser VERIFIED");
        let r = v.validate_asset(&good.id, Some(true), "4 passed", "claude").unwrap();
        assert_eq!(r.status_after, "VERIFIED");
        assert_eq!(v.get_asset(&good.id).unwrap().metadata["last_validation"]["result"], "VERIFIED");
        let bad = v.import_file(&file(src.path(), "Bad.luau", b"local function (\n"), None, None).unwrap().asset;
        assert_eq!(v.validate_asset(&bad.id, Some(true), "", "claude").unwrap().status_after, "FAILED", "sintaxis rota nunca es funcional");
        let r = v.validate_asset(&good.id, Some(false), "1 failed", "claude").unwrap();
        assert_eq!(r.status_after, "FAILED");
        let dep = v.import_file(&file(src.path(), "Uses.luau", b"local X = require(script.Parent.Nope)\nreturn X"), None, None).unwrap().asset;
        assert_eq!(v.validate_asset(&dep.id, Some(true), "", "claude").unwrap().status_after, "PARTIALLY VERIFIED");
        let rbx = v.import_file(&file(src.path(), "S.rbxmx", br#"<roblox version="4"><Item class="Script" referent="A"><Properties><string name="Name">S</string><ProtectedString name="Source">local = </ProtectedString></Properties></Item></roblox>"#), None, None).unwrap().asset;
        assert_eq!(v.validate_asset(&rbx.id, None, "", "claude").unwrap().status_after, "FAILED");
    }

    #[test]
    fn ranked_search() {
        assert_eq!(search_tokens("InventoryUIController v2"), vec!["inventory", "ui", "controller", "inventoryuicontroller", "v", "2", "v2"]);
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        v.import_file(&file(src.path(), "InventoryController.luau", b"local Inventory = {}\nfunction Inventory.AddItem(player, item) end\nreturn Inventory"), None, None).unwrap();
        v.import_file(&file(src.path(), "ShopUI.luau", b"-- tienda\nlocal Inventory = require(script.Parent.InventoryController)\nInventory.AddItem(p, 'Sword')"), None, None).unwrap();
        v.import_file(&file(src.path(), "DoorScript.luau", b"script.Parent.Touched:Connect(function() end)"), None, None).unwrap();
        let r = v.search_ranked(None, None, "inventory add item", 10).unwrap();
        assert_eq!(r[0].asset.name, "InventoryController", "el que se llama así va primero");
        assert_eq!(r.len(), 2, "la puerta no aparece");
        let r = v.search_ranked(None, None, "invent", 10).unwrap();
        assert!(!r.is_empty(), "prefijos");
        assert!(v.search_ranked(None, None, "dragon", 10).unwrap().is_empty());
        assert_eq!(v.search_ranked(None, None, "", 10).unwrap().len(), 3);
    }

    #[test]
    fn folder_import_icons() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let v = Vault::open(root.path()).unwrap();
        fs::create_dir_all(src.path().join("Currencies")).unwrap();
        fs::create_dir_all(src.path().join("UI")).unwrap();
        file(&src.path().join("Currencies"), "Coin.png", b"\x89PNGcoin");
        file(&src.path().join("Currencies"), "CoinBlackStroke.png", b"\x89PNGcoin2");
        file(&src.path().join("UI"), "X.png", b"\x89PNGx");
        file(&src.path().join("UI"), "dup.png", b"\x89PNGx");
        file(src.path(), "License.txt", b"Use in commercial projects");
        file(src.path(), "README.pdf", b"%PDF");
        let r = v.import_folder(src.path(), "Naro", None).unwrap();
        assert_eq!((r.imported, r.duplicates, r.skipped), (3, 1, 2));
        let a = v.search_ranked(Some("texture"), None, "coin black stroke", 5).unwrap();
        assert_eq!(a[0].asset.metadata["variant"], "BlackStroke");
        assert_eq!(a[0].asset.metadata["category"], "Currencies");
        assert_eq!(a[0].asset.metadata["icon"], "Coin");
        assert!(a[0].asset.metadata["license"].as_str().unwrap().contains("commercial"));
        assert_eq!(v.import_folder(src.path(), "Naro", None).unwrap().imported, 0, "repetir no duplica");
    }
}
