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

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/001_init.sql")), (2, include_str!("../migrations/002_library.sql"))];
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
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
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
        self.conn.execute("INSERT INTO activity(at,actor,action,target,detail,destructive) VALUES (?,?,?,?,?,?)",
            params![now(), actor, action, target, detail, destructive as i64])?;
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
            self.log("app", "asset.version", &a.name, &format!("v{}", a.version + 1), false)?;
            aid.to_string()
        } else {
            let (kind, format) = classify(&original);
            let name = Path::new(&original).file_stem().and_then(|s| s.to_str()).unwrap_or(&original).to_string();
            let id = uuid::Uuid::new_v4().to_string();
            let (name, mut meta) = if kind == "script" { let (n, st) = script_type(&name); (n, serde_json::json!({ "script_type": st })) } else { (name, serde_json::json!({})) };
            if let Some(extra) = roblox_meta(src, &format) { meta = extra; }
            self.conn.execute("INSERT INTO assets(id,project_id,kind,name,format,original_name,status,metadata,created_at,updated_at) VALUES (?,?,?,?,?,?, 'PENDING',?,?,?)",
                params![id, project_id, kind, name, format, original, meta.to_string(), t, t])?;
            self.conn.execute("INSERT INTO asset_versions VALUES (?,?,1,?,?,?)", params![uuid::Uuid::new_v4().to_string(), id, hash, "", t])?;
            self.log("app", "asset.import", &original, &hash[..12], false)?;
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
        self.log("app", "asset.status", id, status, false)
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
        self.log("app", "asset.restore", &a.name, &format!("v{v} -> {}", dest.display()), false)?;
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
        self.log("app", "asset.update", name, "", false)?;
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
}
