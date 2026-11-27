//! Las 12 herramientas MCP. Cada una valida su entrada, responde JSON estructurado,
//! devuelve errores controlados y deja registro en la actividad (actor "claude").

use serde_json::{json, Value};
use std::path::PathBuf;
use vault_core::{Asset, Vault, STATUSES};

type R = Result<Value, String>;

fn s<'a>(a: &'a Value, k: &str) -> Option<&'a str> { a.get(k).and_then(Value::as_str).map(str::trim).filter(|x| !x.is_empty()) }
fn req<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> { s(a, k).ok_or_else(|| format!("falta el parámetro '{k}'")) }
fn b(a: &Value, k: &str) -> Option<bool> { a.get(k).and_then(Value::as_bool) }
fn i(a: &Value, k: &str) -> Option<i64> { a.get(k).and_then(Value::as_i64) }
fn e<E: std::fmt::Display>(x: E) -> String { x.to_string() }
fn to<T: serde::Serialize>(x: T) -> Value { serde_json::to_value(x).unwrap_or(Value::Null) }

/// Ruta absoluta y existente (o su padre). Sin `..` para evitar path traversal.
fn abs_path(p: &str, must_be_file: bool) -> Result<PathBuf, String> {
    let path = PathBuf::from(p);
    if !path.is_absolute() { return Err(format!("la ruta debe ser absoluta: {p}")); }
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) { return Err("la ruta no puede contener '..'".into()); }
    if must_be_file && !path.is_file() { return Err(format!("no existe el archivo: {p}")); }
    Ok(path)
}

/// Acepta id o nombre exacto (sin distinguir mayúsculas). Si el nombre es ambiguo, error con candidatos.
fn resolve_asset(v: &Vault, key: &str, kind: Option<&str>) -> Result<Asset, String> {
    if let Ok(a) = v.get_asset(key) { return Ok(a); }
    let hits: Vec<Asset> = v.search_assets(kind, None, None, false).map_err(e)?.into_iter().filter(|a| a.name.eq_ignore_ascii_case(key) || a.original_name.eq_ignore_ascii_case(key)).collect();
    match hits.len() {
        0 => Err(format!("no hay ningún recurso '{key}'. Usa vault_search.")),
        1 => Ok(hits.into_iter().next().unwrap()),
        _ => Err(format!("'{key}' es ambiguo: {}", hits.iter().map(|a| format!("{} ({}, {})", a.id, a.project_name.as_deref().unwrap_or("global"), a.format)).collect::<Vec<_>>().join(", "))),
    }
}
fn resolve_project(v: &Vault, key: &str) -> Result<String, String> {
    v.list_projects().map_err(e)?.into_iter().find(|p| p.id == key || p.name.eq_ignore_ascii_case(key)).map(|p| p.id).ok_or_else(|| format!("no existe el proyecto '{key}'. Usa vault_list_projects."))
}
fn brief(a: &Asset) -> Value {
    json!({ "id": a.id, "name": a.name, "kind": a.kind, "format": a.format, "project": a.project_name, "version": a.version, "status": a.status, "updated_at": a.updated_at, "script_type": a.metadata.get("script_type") })
}

pub fn definitions() -> Value {
    let id = json!({ "type": "string", "description": "id del recurso o su nombre exacto" });
    json!([
      { "name": "vault_search", "description": "Busca recursos existentes (modelos, scripts, configs, animaciones) por nombre y por contenido del código. Úsalo SIEMPRE antes de construir algo.",
        "inputSchema": { "type": "object", "properties": {
          "query": { "type": "string", "description": "texto a buscar (nombre o código)" },
          "kind": { "type": "string", "enum": ["model", "script", "texture", "config", "audio", "animation", "other"] },
          "project": { "type": "string", "description": "id o nombre del proyecto; omítelo para buscar en todos" },
          "in_content": { "type": "boolean", "default": true, "description": "buscar también dentro del código" },
          "include_systems": { "type": "boolean", "default": true } } } },
      { "name": "vault_get_asset", "description": "Ficha completa de un recurso: metadatos, versiones, estado de validación y dependencias.",
        "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] } },
      { "name": "vault_save_asset", "description": "Registra un archivo nuevo que construiste. Si lo probaste, indica el resultado REAL. Probado+OK entra como PARTIALLY VERIFIED (si el usuario tiene activo el registro automático), probado+fallo entra como FAILED y lo no probado queda pendiente de la aprobación del usuario.",
        "inputSchema": { "type": "object", "properties": {
          "path": { "type": "string", "description": "ruta absoluta del archivo" }, "project": { "type": "string" },
          "tested": { "type": "boolean" }, "test_passed": { "type": "boolean" }, "test_output": { "type": "string" }, "note": { "type": "string" } }, "required": ["path", "tested"] } },
      { "name": "vault_get_script", "description": "Devuelve el código de un script (versión actual o una concreta).",
        "inputSchema": { "type": "object", "properties": { "id": id, "version": { "type": "integer" } }, "required": ["id"] } },
      { "name": "vault_get_system", "description": "Devuelve un sistema completo: recursos con rol, instalación, configuración, compatibilidad, pruebas y avisos de dependencias.",
        "inputSchema": { "type": "object", "properties": { "id": { "type": "string", "description": "id o nombre del sistema" }, "include_code": { "type": "boolean", "default": false } }, "required": ["id"] } },
      { "name": "vault_restore_asset", "description": "Copia el original guardado a una carpeta. Nunca sobrescribe: si existe, añade un sufijo. Comprueba la integridad antes.",
        "inputSchema": { "type": "object", "properties": { "id": id, "version": { "type": "integer" }, "dest_dir": { "type": "string", "description": "carpeta absoluta de destino" }, "with_dependencies": { "type": "boolean", "default": false } }, "required": ["id", "dest_dir"] } },
      { "name": "vault_update_asset", "description": "Crea una versión nueva a partir de un archivo (la anterior se conserva) y/o cambia nombre, proyecto o metadatos. El estado solo puede pasar a DEPRECATED o PENDING aquí; para verificar usa vault_validate_asset.",
        "inputSchema": { "type": "object", "properties": { "id": id, "new_file_path": { "type": "string" }, "name": { "type": "string" }, "project": { "type": "string" },
          "metadata": { "type": "object", "description": "se fusiona con los metadatos actuales" }, "status": { "type": "string", "enum": ["DEPRECATED", "PENDING"] } }, "required": ["id"] } },
      { "name": "vault_get_dependencies", "description": "Dependencias directas y en cadena, quién usa el recurso, problemas (faltantes, FAILED, ciclos) y require() sin resolver.",
        "inputSchema": { "type": "object", "properties": { "id": id, "add": { "type": "array", "items": { "type": "string" }, "description": "ids o nombres a registrar como dependencias" } }, "required": ["id"] } },
      { "name": "vault_validate_asset", "description": "Valida integridad, estructura (sintaxis Luau, RBXM, GLB, JSON) y dependencias, y registra el resultado real de pruebas. Sin pruebas el máximo es PARTIALLY VERIFIED.",
        "inputSchema": { "type": "object", "properties": { "id": id, "test_passed": { "type": "boolean" }, "test_output": { "type": "string" } }, "required": ["id"] } },
      { "name": "vault_list_projects", "description": "Lista los juegos/proyectos con cuántos recursos y sistemas tiene cada uno.",
        "inputSchema": { "type": "object", "properties": {} } },
      { "name": "vault_get_project_context", "description": "Memoria de un juego: descripción, arquitectura, decisiones técnicas, errores conocidos, cambios recientes, recursos y sistemas.",
        "inputSchema": { "type": "object", "properties": { "project": { "type": "string" } }, "required": ["project"] } },
      { "name": "vault_log_activity", "description": "Registra en la memoria una decisión técnica, un error conocido, un cambio o una nota de arquitectura (con proyecto), o una entrada de actividad.",
        "inputSchema": { "type": "object", "properties": { "action": { "type": "string" }, "detail": { "type": "string" }, "project": { "type": "string" },
          "note_kind": { "type": "string", "enum": ["decision", "bug", "change", "architecture"] }, "asset_id": { "type": "string" } }, "required": ["action", "detail"] } }
    ])
}

pub fn call(v: &Vault, name: &str, a: &Value) -> R {
    let out = match name {
        "vault_search" => {
            let pid = s(a, "project").map(|p| resolve_project(v, p)).transpose()?;
            let q = s(a, "query");
            let assets = v.search_assets(s(a, "kind"), pid.as_deref(), q, b(a, "in_content").unwrap_or(true)).map_err(e)?;
            let systems = if b(a, "include_systems").unwrap_or(true) { v.list_systems(pid.as_deref(), q).map_err(e)? } else { vec![] };
            v.log("claude", "mcp.search", q.unwrap_or("*"), &format!("{} recursos, {} sistemas", assets.len(), systems.len()), false).map_err(e)?;
            json!({ "count": assets.len(), "assets": assets.iter().take(100).map(brief).collect::<Vec<_>>(),
                    "systems": systems.iter().map(|s| json!({ "id": s.id, "name": s.name, "status": s.status, "members": s.members.len(), "warnings": s.warnings })).collect::<Vec<_>>() })
        }
        "vault_get_asset" => {
            let x = resolve_asset(v, req(a, "id")?, None)?;
            let versions = v.list_versions(&x.id).map_err(e)?;
            let deps = v.dependency_report(&x.id).map_err(e)?;
            v.log_asset("claude", "mcp.get_asset", Some(&x.id), &x.name, "", false).map_err(e)?;
            json!({ "asset": x, "versions": versions, "dependencies": deps.direct.iter().map(|d| brief(&d.asset)).collect::<Vec<_>>(),
                    "dependency_problems": deps.problems, "used_by": deps.dependents.iter().map(brief).collect::<Vec<_>>() })
        }
        "vault_save_asset" => {
            let path = abs_path(req(a, "path")?, true)?;
            let tested = b(a, "tested").ok_or("falta el parámetro 'tested' (¿lo probaste?)")?;
            let passed = b(a, "test_passed");
            if tested && passed.is_none() { return Err("si tested=true indica test_passed con el resultado real".into()); }
            let pid = s(a, "project").map(|p| resolve_project(v, p)).transpose()?;
            let c = v.capture(&path, pid.as_deref(), tested, if tested { passed } else { None }, s(a, "test_output").unwrap_or(""), s(a, "note").unwrap_or(""), "claude").map_err(e)?;
            let asset = c.asset_id.as_deref().map(|i| v.get_asset(i)).transpose().map_err(e)?;
            json!({ "capture": c, "asset": asset.as_ref().map(brief),
                    "message": match c.state.as_str() { "approved" => "Guardado en la bóveda.", _ => "Pendiente de aprobación del usuario en ROBLOX VAULT (Activity → Captura)." } })
        }
        "vault_get_script" => {
            let x = resolve_asset(v, req(a, "id")?, None)?;
            let code = v.get_text(&x.id, i(a, "version")).map_err(e)?.ok_or("este recurso no es un archivo de texto")?;
            v.log_asset("claude", "mcp.get_script", Some(&x.id), &x.name, "", false).map_err(e)?;
            json!({ "asset": brief(&x), "version": i(a, "version").unwrap_or(x.version), "code": code })
        }
        "vault_get_system" => {
            let key = req(a, "id")?;
            let sys = v.list_systems(None, None).map_err(e)?.into_iter().find(|s| s.id == key || s.name.eq_ignore_ascii_case(key)).ok_or_else(|| format!("no existe el sistema '{key}'"))?;
            let with_code = b(a, "include_code").unwrap_or(false);
            let members: Vec<Value> = sys.members.iter().map(|m| {
                let mut o = json!({ "role": m.role, "asset": brief(&m.asset) });
                if with_code { o["code"] = json!(v.get_text(&m.asset.id, None).ok().flatten()); }
                o
            }).collect();
            v.log("claude", "mcp.get_system", &sys.name, "", false).map_err(e)?;
            json!({ "id": sys.id, "name": sys.name, "status": sys.status, "description": sys.description, "install": sys.install, "config": sys.config,
                    "compatibility": sys.compatibility, "tests": sys.tests, "members": members, "warnings": sys.warnings, "ready_to_restore": !sys.warnings.iter().any(|w| w.contains("falta")) })
        }
        "vault_restore_asset" => {
            let x = resolve_asset(v, req(a, "id")?, None)?;
            let dest = abs_path(req(a, "dest_dir")?, false)?;
            let mut written = vec![v.restore_asset(&x.id, i(a, "version"), &dest).map_err(e)?];
            let mut problems = vec![];
            if b(a, "with_dependencies").unwrap_or(false) {
                let rep = v.dependency_report(&x.id).map_err(e)?;
                problems = rep.problems.clone();
                if problems.iter().any(|p| p.contains("falta")) { return Err(format!("faltan dependencias: {}", problems.join("; "))); }
                for d in rep.all { written.push(v.restore_asset(&d.asset.id, None, &dest).map_err(e)?); }
            }
            json!({ "written": written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(), "dependency_warnings": problems, "status": x.status,
                    "warning": if x.status == "FAILED" { Some("Este recurso está marcado como FAILED") } else { None } })
        }
        "vault_update_asset" => {
            let x = resolve_asset(v, req(a, "id")?, None)?;
            let mut new_version = None;
            if let Some(p) = s(a, "new_file_path") {
                let r = v.import_file(&abs_path(p, true)?, None, Some(&x.id)).map_err(e)?;
                new_version = Some(json!({ "created": r.new_version, "version": r.asset.version, "identical_to_current": !r.new_version }));
            }
            let pid = match s(a, "project") { Some(p) => Some(Some(resolve_project(v, p)?)), None => None };
            let meta = match a.get("metadata") {
                Some(Value::Object(m)) => { let mut cur = v.get_asset(&x.id).map_err(e)?.metadata; if let Some(o) = cur.as_object_mut() { for (k, val) in m { o.insert(k.clone(), val.clone()); } } Some(cur) }
                Some(_) => return Err("metadata debe ser un objeto".into()), None => None,
            };
            let updated = v.update_asset(&x.id, s(a, "name"), pid.as_ref().map(|p| p.as_deref()), meta.as_ref()).map_err(e)?;
            if let Some(st) = s(a, "status") {
                if st != "DEPRECATED" && st != "PENDING" { return Err(format!("estado '{st}' no permitido aquí; usa vault_validate_asset (válidos: {})", STATUSES.join(", "))); }
                v.set_status(&x.id, st).map_err(e)?;
            }
            v.log_asset("claude", "mcp.update_asset", Some(&x.id), &x.name, "", false).map_err(e)?;
            json!({ "asset": brief(&v.get_asset(&updated.id).map_err(e)?), "new_version": new_version })
        }
        "vault_get_dependencies" => {
            let x = resolve_asset(v, req(a, "id")?, None)?;
            if let Some(Value::Array(add)) = a.get("add") {
                for d in add { let k = d.as_str().ok_or("'add' debe ser una lista de textos")?; let dep = resolve_asset(v, k, None)?; v.add_dependency(&x.id, &dep.id, "mcp").map_err(e)?; }
            }
            let r = v.dependency_report(&x.id).map_err(e)?;
            json!({ "asset": brief(&x), "direct": r.direct.iter().map(|d| brief(&d.asset)).collect::<Vec<_>>(),
                    "all": r.all.iter().map(|d| json!({ "depth": d.depth, "asset": brief(&d.asset) })).collect::<Vec<_>>(),
                    "used_by": r.dependents.iter().map(brief).collect::<Vec<_>>(), "problems": r.problems,
                    "unresolved_requires": r.unresolved_requires, "suggested_from_code": r.suggestions.iter().map(brief).collect::<Vec<_>>() })
        }
        "vault_validate_asset" => {
            let x = resolve_asset(v, req(a, "id")?, None)?;
            to(v.validate_asset(&x.id, b(a, "test_passed"), s(a, "test_output").unwrap_or(""), "claude").map_err(e)?)
        }
        "vault_list_projects" => {
            let ps = v.list_projects().map_err(e)?;
            let out: Vec<Value> = ps.iter().map(|p| {
                let n = v.search_assets(None, Some(&p.id), None, false).map(|x| x.len()).unwrap_or(0);
                let sy = v.list_systems(Some(&p.id), None).map(|x| x.len()).unwrap_or(0);
                json!({ "id": p.id, "name": p.name, "description": p.description, "assets": n, "systems": sy, "updated_at": p.updated_at })
            }).collect();
            json!({ "projects": out, "global_assets": v.search_assets(None, None, None, false).map_err(e)?.iter().filter(|a| a.project_id.is_none()).count() })
        }
        "vault_get_project_context" => {
            let pid = resolve_project(v, req(a, "project")?)?;
            let c = v.project_context(&pid).map_err(e)?;
            v.log("claude", "mcp.project_context", &c.project.name, "", false).map_err(e)?;
            let by = |k: &str| c.notes.iter().filter(|n| n.kind == k).map(|n| json!({ "text": n.text, "at": n.created_at })).collect::<Vec<_>>();
            json!({ "project": c.project, "technical_decisions": by("decision"), "known_bugs": by("bug"), "recent_changes": by("change"), "architecture_notes": by("architecture"),
                    "assets": c.assets.iter().map(brief).collect::<Vec<_>>(),
                    "systems": c.systems.iter().map(|s| json!({ "id": s.id, "name": s.name, "status": s.status, "members": s.members.len() })).collect::<Vec<_>>(),
                    "recent_activity": c.recent.iter().take(15).map(|r| json!({ "at": r.at, "action": r.action, "target": r.target })).collect::<Vec<_>>() })
        }
        "vault_log_activity" => {
            let action = req(a, "action")?; let detail = req(a, "detail")?;
            if action.len() > 80 { return Err("'action' demasiado largo (máx. 80)".into()); }
            let mut note = None;
            if let Some(k) = s(a, "note_kind") {
                let pid = resolve_project(v, s(a, "project").ok_or("para note_kind hace falta 'project'")?)?;
                note = Some(v.add_project_note(&pid, k, detail).map_err(e)?);
            }
            let aid = s(a, "asset_id").map(|k| resolve_asset(v, k, None).map(|x| x.id)).transpose()?;
            v.log_asset("claude", action, aid.as_deref(), s(a, "project").unwrap_or(""), detail, false).map_err(e)?;
            json!({ "logged": true, "project_note": note })
        }
        _ => return Err(format!("herramienta desconocida: {name}")),
    };
    Ok(out)
}

