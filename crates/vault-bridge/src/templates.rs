//! Plantillas BASE incluidas en la app (carpeta `templates/`): se instalan en la bóveda como
//! sistemas completos. Sus pruebas se ejecutan con Luau real en cada compilación (CI) antes de publicar.

use include_dir::{include_dir, Dir};
use serde_json::{json, Value};
use std::path::Path;
use vault_core::{SystemInput, Vault};

pub static TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../templates");

pub fn manifest() -> Result<Value, String> {
    let f = TEMPLATES.get_file("manifest.json").ok_or("falta templates/manifest.json")?;
    serde_json::from_slice(f.contents()).map_err(|e| e.to_string())
}

/// Instala o actualiza las plantillas. Idempotente: lo que no cambió no crea versiones nuevas.
pub fn install(v: &Vault, tmp: &Path) -> Result<Value, String> {
    let m = manifest()?;
    let tested_by = m["tested_by"].as_str().unwrap_or("").to_string();
    let mut report = vec![];
    let existing = v.list_systems(None, None).map_err(|e| e.to_string())?;
    for sys in m["systems"].as_array().ok_or("manifest sin systems")? {
        let name = sys["name"].as_str().ok_or("sistema sin nombre")?;
        let input = SystemInput {
            name: name.into(), project_id: None,
            description: sys["description"].as_str().unwrap_or("").into(), install: sys["install"].as_str().unwrap_or("").into(),
            config: sys["config"].as_str().unwrap_or("").into(), compatibility: sys["compatibility"].as_str().unwrap_or("").into(),
            tests: sys["tests"].as_str().unwrap_or("").into(),
        };
        let current = existing.iter().find(|s| s.name == name);
        let sys_id = match current { Some(s) => v.update_system(&s.id, &input).map_err(|e| e.to_string())?.id, None => v.create_system(&input).map_err(|e| e.to_string())?.id };
        let members = v.get_system(&sys_id).map_err(|e| e.to_string())?.members;
        let (mut added, mut updated) = (0, 0);
        for f in sys["files"].as_array().ok_or("sistema sin files")? {
            let rel = f["file"].as_str().ok_or("archivo sin ruta")?;
            let file = TEMPLATES.get_file(rel).ok_or_else(|| format!("falta templates/{rel}"))?;
            let fname = Path::new(rel).file_name().unwrap().to_string_lossy().to_string();
            let dir = tmp.join(uuid::Uuid::new_v4().to_string());
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(&fname);
            std::fs::write(&path, file.contents()).map_err(|e| e.to_string())?;
            let prev = members.iter().find(|m| m.asset.original_name == fname);
            let r = v.import_file(&path, None, prev.map(|p| p.asset.id.as_str())).map_err(|e| e.to_string());
            let _ = std::fs::remove_dir_all(&dir);
            let r = r?;
            let id = r.asset.id.clone();
            if prev.is_none() { v.add_system_member(&sys_id, &id, f["role"].as_str().unwrap_or("")).map_err(|e| e.to_string())?; added += 1; }
            else if r.new_version { updated += 1; }
            let mut meta = v.get_asset(&id).map_err(|e| e.to_string())?.metadata;
            if let Some(o) = meta.as_object_mut() {
                o.insert("studio_location".into(), json!(f["location"]));
                o.insert("template".into(), json!(name));
            }
            v.update_asset(&id, None, None, Some(&meta)).map_err(|e| e.to_string())?;
            // las piezas con pruebas Luau reales (spec y módulo probado) → VERIFIED; el resto solo estructura
            let tested = f.get("spec_of").is_some() || sys["files"].as_array().unwrap().iter().any(|o| o["spec_of"].as_str() == Some(rel));
            let a = v.get_asset(&id).map_err(|e| e.to_string())?;
            if a.status != "VERIFIED" || r.new_version {
                v.validate_asset(&id, if tested { Some(true) } else { None }, if tested { &tested_by } else { "" }, "templates").map_err(|e| e.to_string())?;
            }
        }
        let st = v.get_system(&sys_id).map_err(|e| e.to_string())?;
        let all_verified = st.members.iter().all(|m| m.asset.status == "VERIFIED");
        v.set_system_status(&sys_id, if all_verified { "VERIFIED" } else { "PARTIALLY VERIFIED" }).map_err(|e| e.to_string())?;
        report.push(json!({ "system": name, "id": sys_id, "added": added, "updated": updated, "members": st.members.len() }));
    }
    v.log("app", "templates.install", "BASE", &format!("{} sistemas", report.len()), false).map_err(|e| e.to_string())?;
    Ok(json!({ "systems": report }))
}
