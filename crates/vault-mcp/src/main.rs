//! Servidor MCP de ROBLOX VAULT (stdio, JSON-RPC 2.0, un mensaje por línea).
//! Comparte la misma bóveda que la app: `ROBLOX_VAULT_HOME` o `Documentos/RobloxVault/data`.

mod tools;

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use vault_core::Vault;

pub const PROTOCOL: &str = "2025-06-18";

pub const INSTRUCTIONS: &str = "ROBLOX VAULT es la memoria técnica permanente de los juegos de Roblox del usuario.
Protocolo obligatorio:
1. Antes de crear un sistema, script o modelo: vault_search (por nombre y por contenido) y vault_get_project_context.
2. Si existe algo reutilizable: examínalo (vault_get_script / vault_get_system / vault_get_asset), comprueba dependencias (vault_get_dependencies) y compatibilidad.
3. Reutiliza con vault_restore_asset; si hay que adaptarlo, guarda una versión nueva con vault_update_asset (nunca reescribas desde cero algo que ya existe).
4. Registra lo que construyas con vault_save_asset, indicando si lo probaste y el resultado REAL de las pruebas.
5. Tras ejecutar pruebas, registra el resultado con vault_validate_asset. Nunca marques algo como funcional sin pruebas reales.
6. Registra decisiones técnicas y errores conocidos con vault_log_activity.";

pub fn default_root() -> PathBuf {
    std::env::var_os("ROBLOX_VAULT_HOME").map(PathBuf::from)
        .unwrap_or_else(|| dirs::document_dir().unwrap_or_else(|| PathBuf::from(".")).join("RobloxVault").join("data"))
}

/// Procesa un mensaje JSON-RPC. Devuelve `None` para notificaciones.
pub fn handle(vault: &Vault, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let id = id?; // notificaciones (initialized, cancelled…) no llevan respuesta
    let result = match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL);
            Ok(json!({
                "protocolVersion": if ["2024-11-05", "2025-03-26", "2025-06-18"].contains(&asked) { asked } else { PROTOCOL },
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "roblox-vault", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::definitions() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Ok(match tools::call(vault, name, &args) {
                Ok(v) => json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() }], "structuredContent": v, "isError": false }),
                Err(e) => {
                    let _ = vault.log("claude", "mcp.error", name, &e, false);
                    json!({ "content": [{ "type": "text", "text": format!("Error: {e}") }], "isError": true })
                }
            })
        }
        _ => Err(json!({ "code": -32601, "message": format!("método no soportado: {method}") })),
    };
    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
    })
}

fn main() {
    let root = default_root();
    let vault = match Vault::open(&root) {
        Ok(v) => v,
        Err(e) => { eprintln!("ROBLOX VAULT: no se pudo abrir la bóveda en {}: {e}", root.display()); std::process::exit(1) }
    };
    eprintln!("ROBLOX VAULT MCP listo · bóveda: {}", root.display());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() { continue; }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => { let r: Vec<Value> = batch.iter().filter_map(|m| handle(&vault, m)).collect(); if r.is_empty() { None } else { Some(Value::Array(r)) } }
            Ok(msg) => handle(&vault, &msg),
            Err(e) => Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON inválido: {e}") } })),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}
