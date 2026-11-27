//! Puente local entre el plugin de Roblox Studio y la bóveda.
//! HTTP en 127.0.0.1 (solo local), autenticado con un token que la app inserta en el plugin al instalarlo.
//! Los binarios viajan en base64 (EncodingService en Studio).

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use vault_core::{roblox, Asset, Vault};

pub const DEFAULT_PORT: u16 = 47515;
pub const PLUGIN_SOURCE: &str = include_str!("../../../plugin/RobloxVault.plugin.luau");
const MAX_BODY: usize = 64 * 1024 * 1024;

pub struct Bridge { pub vault: Mutex<Vault>, pub token: String, pub tmp: PathBuf }

pub struct Reply { pub status: u16, pub body: Value }
fn ok(v: Value) -> Reply { Reply { status: 200, body: v } }
fn err(status: u16, m: impl Into<String>) -> Reply { Reply { status, body: json!({ "error": m.into() }) } }

fn brief(a: &Asset) -> Value {
    json!({ "id": a.id, "name": a.name, "kind": a.kind, "format": a.format, "project": a.project_name, "version": a.version, "status": a.status,
            "script_type": a.metadata.get("script_type"), "insertable": matches!(a.format.as_str(), "rbxm" | "rbxmx" | "lua" | "luau") })
}

fn query(url: &str) -> Vec<(String, String)> {
    url.split_once('?').map(|(_, q)| q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), decode(v))).collect()).unwrap_or_default()
}
fn decode(s: &str) -> String {
    let b = s.as_bytes(); let mut out = vec![]; let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => { out.push(b' '); i += 1 }
            b'%' if i + 2 < b.len() => { if let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(x) } i += 3 }
            c => { out.push(c); i += 1 }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Nombre de archivo seguro (sin separadores ni caracteres reservados de Windows).
pub fn safe_name(n: &str) -> String {
    let s: String = n.chars().map(|c| if c.is_alphanumeric() || " -_.()".contains(c) { c } else { '_' }).collect();
    let s = s.trim_matches(|c| c == '.' || c == ' ').chars().take(80).collect::<String>();
    if s.is_empty() { "Recurso".into() } else { s }
}

impl Bridge {
    pub fn new(vault: Vault, token: String, tmp: PathBuf) -> Self { Bridge { vault: Mutex::new(vault), token, tmp } }

    fn project_for(&self, v: &Vault, place: &Value) -> Result<Option<String>, String> {
        let Some(pid) = place.get("place_id").and_then(Value::as_i64).filter(|p| *p > 0) else { return Ok(None) };
        Ok(v.setting(&format!("place:{pid}")).map_err(|e| e.to_string())?)
    }

    /// Guarda bytes recibidos de Studio como recurso (o versión nueva si se indica `update_id`).
    fn store(&self, v: &Vault, file_name: &str, bytes: &[u8], project: Option<String>, update_id: Option<&str>, path: &str) -> Result<Value, String> {
        std::fs::create_dir_all(&self.tmp).map_err(|e| e.to_string())?;
        let dir = self.tmp.join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let f = dir.join(file_name);
        std::fs::write(&f, bytes).map_err(|e| e.to_string())?;
        let r = v.import_file(&f, project.as_deref(), update_id);
        let _ = std::fs::remove_dir_all(&dir);
        let r = r.map_err(|e| e.to_string())?;
        if !path.is_empty() {
            let mut m = r.asset.metadata.clone();
            if let Some(o) = m.as_object_mut() { o.insert("studio_path".into(), json!(path)); }
            v.update_asset(&r.asset.id, None, None, Some(&m)).map_err(|e| e.to_string())?;
        }
        v.log_asset("studio", "studio.send", Some(&r.asset.id), &r.asset.name, path, false).map_err(|e| e.to_string())?;
        Ok(json!({ "asset": brief(&v.get_asset(&r.asset.id).map_err(|e| e.to_string())?), "duplicate": r.duplicate_of.is_some() && !r.new_version, "new_version": r.new_version }))
    }

    pub fn handle(&self, method: &str, url: &str, token: Option<&str>, body: &[u8]) -> Reply {
        let path = url.split('?').next().unwrap_or("");
        if path == "/ping" { return ok(json!({ "name": "ROBLOX VAULT", "api": 1 })); }
        if token != Some(self.token.as_str()) { return err(401, "token inválido: reinstala el plugin desde ROBLOX VAULT → Settings"); }
        if body.len() > MAX_BODY { return err(413, "demasiado grande"); }
        let v = match self.vault.lock() { Ok(v) => v, Err(_) => return err(500, "bóveda bloqueada") };
        let json_body = || serde_json::from_slice::<Value>(body).map_err(|e| format!("JSON inválido: {e}"));
        let q = query(url);
        let qp = |k: &str| q.iter().find(|(a, _)| a == k).map(|(_, b)| b.clone()).filter(|s| !s.is_empty());
        let seg: Vec<&str> = path.trim_matches('/').split('/').collect();
        let res: Result<Value, String> = (|| match (method, seg.as_slice()) {
            ("GET", ["search"]) => {
                let pid = qp("place_id").and_then(|p| p.parse::<i64>().ok()).map(|p| json!({ "place_id": p }));
                let project = match (qp("scope").as_deref(), pid) { (Some("project"), Some(p)) => self.project_for(&v, &p)?, _ => None };
                let list: Vec<Asset> = match qp("q") {
                    Some(q) => v.search_ranked(qp("kind").as_deref(), project.as_deref(), &q, 60).map_err(|e| e.to_string())?.into_iter().map(|r| r.asset).collect(),
                    None => v.search_assets(qp("kind").as_deref(), project.as_deref(), None, false).map_err(|e| e.to_string())?,
                };
                Ok(json!({ "assets": list.iter().take(60).map(brief).collect::<Vec<_>>() }))
            }
            ("GET", ["asset", id, "rbxm"]) => {
                let a = v.get_asset(id).map_err(|e| e.to_string())?;
                if !matches!(a.format.as_str(), "rbxm" | "rbxmx") { return Err(format!("{} no es un modelo de Roblox (.rbxm/.rbxmx)", a.format)); }
                let bytes = roblox::to_rbxm(&v.read_blob(id, None).map_err(|e| e.to_string())?, &a.format)?;
                v.log_asset("studio", "studio.insert", Some(id), &a.name, "modelo", false).map_err(|e| e.to_string())?;
                Ok(json!({ "asset": brief(&a), "rbxm_base64": B64.encode(bytes) }))
            }
            ("GET", ["asset", id, "script"]) => {
                let a = v.get_asset(id).map_err(|e| e.to_string())?;
                let code = v.get_text(id, None).map_err(|e| e.to_string())?.ok_or("no es un script de texto")?;
                let syntax = vault_core::luau_syntax_errors(&code);
                v.log_asset("studio", "studio.insert", Some(id), &a.name, "script", false).map_err(|e| e.to_string())?;
                Ok(json!({ "asset": brief(&a), "source": code, "class": a.metadata.get("script_type").and_then(Value::as_str).unwrap_or("ModuleScript"), "syntax_errors": syntax }))
            }
            ("POST", ["project", "link"]) => {
                let b = json_body()?;
                let place = b.get("place_id").and_then(Value::as_i64).filter(|p| *p > 0).ok_or("guarda el lugar en Roblox para tener un PlaceId")?;
                let name = b.get("name").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).ok_or("falta 'name'")?;
                let existing = v.list_projects().map_err(|e| e.to_string())?.into_iter().find(|p| p.name.eq_ignore_ascii_case(name));
                let p = match existing { Some(p) => p, None => v.create_project(name, "Vinculado desde Roblox Studio").map_err(|e| e.to_string())? };
                v.set_setting(&format!("place:{place}"), &p.id).map_err(|e| e.to_string())?;
                Ok(json!({ "project": { "id": p.id, "name": p.name } }))
            }
            ("GET", ["project"]) => {
                let pid = qp("place_id").and_then(|p| p.parse::<i64>().ok()).ok_or("falta place_id")?;
                let id = self.project_for(&v, &json!({ "place_id": pid }))?;
                let p = id.and_then(|i| v.list_projects().ok()?.into_iter().find(|p| p.id == i));
                Ok(json!({ "project": p.map(|p| json!({ "id": p.id, "name": p.name })) }))
            }
            ("POST", ["model"]) => {
                let b = json_body()?;
                let name = safe_name(b.get("name").and_then(Value::as_str).unwrap_or("Modelo"));
                let data = B64.decode(b.get("rbxm_base64").and_then(Value::as_str).ok_or("falta rbxm_base64")?).map_err(|e| format!("base64 inválido: {e}"))?;
                roblox::parse(&data, "rbxm").map_err(|e| format!("Studio envió un RBXM ilegible: {e}"))?;
                let project = self.project_for(&v, &b)?;
                self.store(&v, &format!("{name}.rbxm"), &data, project, b.get("update_id").and_then(Value::as_str), b.get("path").and_then(Value::as_str).unwrap_or(""))
            }
            ("POST", ["script"]) => {
                let b = json_body()?;
                let name = safe_name(b.get("name").and_then(Value::as_str).unwrap_or("Script"));
                let source = b.get("source").and_then(Value::as_str).ok_or("falta source")?;
                let suffix = match b.get("class").and_then(Value::as_str) { Some("Script") => ".server", Some("LocalScript") => ".client", _ => "" };
                let project = self.project_for(&v, &b)?;
                let mut r = self.store(&v, &format!("{name}{suffix}.luau"), source.as_bytes(), project, b.get("update_id").and_then(Value::as_str), b.get("path").and_then(Value::as_str).unwrap_or(""))?;
                r["syntax_errors"] = json!(vault_core::luau_syntax_errors(source));
                Ok(r)
            }
            ("POST", ["log"]) => {
                let b = json_body()?;
                let action = b.get("action").and_then(Value::as_str).unwrap_or("studio.change");
                v.log("studio", &format!("studio.{}", action.trim_start_matches("studio.")).chars().take(80).collect::<String>(),
                      b.get("target").and_then(Value::as_str).unwrap_or(""), b.get("detail").and_then(Value::as_str).unwrap_or(""), false).map_err(|e| e.to_string())?;
                Ok(json!({ "logged": true }))
            }
            _ => Err(format!("ruta desconocida: {method} {path}")),
        })();
        match res { Ok(v) => ok(v), Err(e) if e.starts_with("ruta desconocida") => err(404, e), Err(e) => err(400, e) }
    }
}

/// Arranca el servidor en un hilo. Devuelve el puerto usado.
pub fn spawn(bridge: std::sync::Arc<Bridge>, port: u16) -> Result<u16, String> {
    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| format!("no se pudo abrir 127.0.0.1:{port}: {e}"))?;
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let mut body = vec![];
            let too_big = req.body_length().map_or(false, |l| l > MAX_BODY);
            if !too_big { let r: &mut dyn std::io::Read = req.as_reader(); let _ = std::io::Read::read_to_end(&mut std::io::Read::take(r, MAX_BODY as u64 + 1), &mut body); }
            let token = req.headers().iter().find(|h| h.field.equiv("X-Vault-Token")).map(|h| h.value.as_str().to_string());
            let reply = if too_big { err(413, "demasiado grande") } else { bridge.handle(req.method().as_str(), req.url(), token.as_deref(), &body) };
            let resp = tiny_http::Response::from_string(reply.body.to_string()).with_status_code(reply.status)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap());
            let _ = req.respond(resp);
        }
    });
    Ok(port)
}

/// Inserta la configuración (puerto y token) en el código del plugin.
pub fn plugin_with_config(port: u16, token: &str) -> String {
    PLUGIN_SOURCE.replace("__VAULT_PORT__", &port.to_string()).replace("__VAULT_TOKEN__", token)
}

/// Carpeta de plugins locales de Roblox Studio en Windows (%LOCALAPPDATA%\Roblox\Plugins).
pub fn studio_plugins_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| Path::new(&d).join("Roblox").join("Plugins"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge() -> (Bridge, tempfile::TempDir) {
        let d = tempfile::tempdir().unwrap();
        let v = Vault::open(d.path().join("vault")).unwrap();
        (Bridge::new(v, "secret".into(), d.path().join("tmp")), d)
    }

    #[test]
    fn auth_and_roundtrip() {
        let (b, _d) = bridge();
        assert_eq!(b.handle("GET", "/ping", None, b"").status, 200);
        assert_eq!(b.handle("GET", "/search", None, b"").status, 401);
        assert_eq!(b.handle("GET", "/search", Some("wrong"), b"").status, 401);
        let t = Some("secret");
        // vincular proyecto por PlaceId
        assert_eq!(b.handle("POST", "/project/link", t, br#"{"place_id":0,"name":"X"}"#).status, 400);
        let r = b.handle("POST", "/project/link", t, br#"{"place_id":123,"name":"Rift Tycoon"}"#);
        assert_eq!(r.status, 200);
        assert_eq!(b.handle("GET", "/project?place_id=123", t, b"").body["project"]["name"], "Rift Tycoon");

        // enviar script desde Studio
        let s = b.handle("POST", "/script", t, br#"{"name":"Inventory/Controller","class":"Script","source":"print('hola')","place_id":123,"path":"ServerScriptService.Inventory"}"#);
        assert_eq!(s.status, 200, "{}", s.body);
        assert_eq!(s.body["asset"]["name"], "Inventory_Controller");
        assert_eq!(s.body["asset"]["script_type"], "Script");
        assert_eq!(s.body["asset"]["project"], "Rift Tycoon");
        let id = s.body["asset"]["id"].as_str().unwrap().to_string();
        // nueva versión
        let s2 = b.handle("POST", "/script", t, format!(r#"{{"name":"Inventory/Controller","class":"Script","source":"print('v2')","update_id":"{id}"}}"#).as_bytes());
        assert_eq!(s2.body["asset"]["version"], 2);
        // buscar dentro del proyecto y recuperar
        let found = b.handle("GET", "/search?q=v2&scope=project&place_id=123", t, b"");
        assert_eq!(found.body["assets"].as_array().unwrap().len(), 1);
        let code = b.handle("GET", &format!("/asset/{id}/script"), t, b"");
        assert_eq!(code.body["source"], "print('v2')");
        assert_eq!(code.body["class"], "Script");

        // modelo: rbxmx en la bóveda → rbxm binario para Studio → de vuelta
        let xml = r#"<roblox version="4"><Item class="Part" referent="A"><Properties><string name="Name">Block</string></Properties></Item></roblox>"#;
        let m = b.handle("POST", "/model", t, json!({ "name": "Block", "rbxm_base64": B64.encode(roblox::to_rbxm(xml.as_bytes(), "rbxmx").unwrap()) }).to_string().as_bytes());
        assert_eq!(m.status, 200, "{}", m.body);
        let mid = m.body["asset"]["id"].as_str().unwrap();
        let back = b.handle("GET", &format!("/asset/{mid}/rbxm"), t, b"");
        let bytes = B64.decode(back.body["rbxm_base64"].as_str().unwrap()).unwrap();
        assert_eq!(roblox::parse(&bytes, "rbxm").unwrap().parts.len(), 1);
        // basura rechazada
        assert_eq!(b.handle("POST", "/model", t, json!({ "name": "X", "rbxm_base64": B64.encode(b"nope") }).to_string().as_bytes()).status, 400);
        assert_eq!(b.handle("GET", &format!("/asset/{id}/rbxm"), t, b"").status, 400);
        assert_eq!(b.handle("POST", "/log", t, br#"{"action":"edit","target":"Workspace.Map"}"#).status, 200);
        assert_eq!(b.handle("DELETE", "/asset/x", t, b"").status, 404);
        let n = safe_name("..\\..//evil:name*"); assert!(!n.contains('/') && !n.contains('\\') && !n.contains(':') && !n.starts_with('.'), "{n}");
    }

    #[test]
    fn plugin_source_is_valid_luau_and_configurable() {
        let src = plugin_with_config(47515, "abc123");
        assert!(src.contains("\"abc123\"") && src.contains("47515"));
        let errs: Vec<String> = full_moon::parse_fallible(&src, full_moon::LuaVersion::luau()).errors().iter().map(|e| e.to_string()).collect();
        assert!(errs.is_empty(), "el plugin tiene errores de sintaxis: {errs:?}");
    }

    #[test]
    fn real_http_server() {
        let (b2, _d2) = bridge();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let p = listener.local_addr().unwrap().port(); drop(listener);
        spawn(std::sync::Arc::new(b2), p).unwrap();
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", p)).unwrap();
        write!(s, "GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = String::new(); s.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 200") && out.contains("ROBLOX VAULT"), "{out}");
    }
}
