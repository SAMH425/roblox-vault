//! Prueba de punta a punta: lanza el binario real y habla MCP por stdio.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

struct Client { stdin: std::process::ChildStdin, out: BufReader<std::process::ChildStdout>, n: i64 }
impl Client {
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.n += 1;
        writeln!(self.stdin, "{}", json!({ "jsonrpc": "2.0", "id": self.n, "method": method, "params": params })).unwrap();
        let mut line = String::new(); self.out.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["id"], self.n);
        v
    }
    fn tool(&mut self, name: &str, args: Value) -> Value { self.rpc("tools/call", json!({ "name": name, "arguments": args }))["result"].clone() }
}

#[test]
fn full_protocol_roundtrip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let script = work.path().join("InventoryController.server.luau");
    std::fs::write(&script, "local Cfg = require(script.Parent.ItemConfig)\nreturn Cfg").unwrap();
    let cfg = work.path().join("ItemConfig.luau");
    std::fs::write(&cfg, "return { Sword = 10 }").unwrap();
    let broken = work.path().join("Broken.luau");
    std::fs::write(&broken, "local = ").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_vault-mcp")).env("ROBLOX_VAULT_HOME", home.path())
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let mut c = Client { stdin: child.stdin.take().unwrap(), out: BufReader::new(child.stdout.take().unwrap()), n: 0 };
    let r = c.rpc("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "1" } }));
    assert_eq!(r["result"]["serverInfo"]["name"], "roblox-vault");
    assert!(r["result"]["instructions"].as_str().unwrap().contains("vault_search"));
    // notificación: sin respuesta
    writeln!(c.stdin, "{}", json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).unwrap();
    let tools = c.rpc("tools/list", json!({}));
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 12);

    // registrar con pruebas superadas → automático
    let saved = c.tool("vault_save_asset", json!({ "path": cfg.to_str().unwrap(), "tested": true, "test_passed": true, "test_output": "ok" }));
    assert_eq!(saved["isError"], false);
    assert_eq!(saved["structuredContent"]["asset"]["status"], "PARTIALLY VERIFIED");
    // sin probar → pendiente
    let pend = c.tool("vault_save_asset", json!({ "path": script.to_str().unwrap(), "tested": false }));
    assert_eq!(pend["structuredContent"]["capture"]["state"], "pending");
    // errores controlados
    assert_eq!(c.tool("vault_save_asset", json!({ "path": "relativo.luau", "tested": false }))["isError"], true);
    assert_eq!(c.tool("vault_save_asset", json!({ "path": cfg.to_str().unwrap(), "tested": true }))["isError"], true);
    assert_eq!(c.tool("vault_get_script", json!({ "id": "NoExiste" }))["isError"], true);
    assert_eq!(c.tool("nope", json!({}))["isError"], true);

    // búsqueda por contenido
    let found = c.tool("vault_search", json!({ "query": "Sword" }));
    assert_eq!(found["structuredContent"]["count"], 1);
    let code = c.tool("vault_get_script", json!({ "id": "ItemConfig" }));
    assert_eq!(code["structuredContent"]["code"], "return { Sword = 10 }");

    // validación: con pruebas → VERIFIED; sintaxis rota → FAILED
    let val = c.tool("vault_validate_asset", json!({ "id": "ItemConfig", "test_passed": true, "test_output": "3 passed" }));
    assert_eq!(val["structuredContent"]["status_after"], "VERIFIED");
    let bad = c.tool("vault_save_asset", json!({ "path": broken.to_str().unwrap(), "tested": true, "test_passed": true }));
    let bad_id = bad["structuredContent"]["asset"]["id"].as_str().unwrap().to_string();
    assert_eq!(c.tool("vault_validate_asset", json!({ "id": bad_id, "test_passed": true }))["structuredContent"]["status_after"], "FAILED");

    // no se puede marcar VERIFIED a mano
    assert_eq!(c.tool("vault_update_asset", json!({ "id": "ItemConfig", "status": "VERIFIED" }))["isError"], true);
    // nueva versión
    std::fs::write(&cfg, "return { Sword = 12 }").unwrap();
    let up = c.tool("vault_update_asset", json!({ "id": "ItemConfig", "new_file_path": cfg.to_str().unwrap(), "metadata": { "notes": "balance" } }));
    assert_eq!(up["structuredContent"]["new_version"]["version"], 2);
    assert_eq!(up["structuredContent"]["asset"]["status"], "PENDING", "una versión nueva vuelve a PENDING");

    // restaurar sin sobrescribir
    let dest = work.path().join("restore");
    let r1 = c.tool("vault_restore_asset", json!({ "id": "ItemConfig", "version": 1, "dest_dir": dest.to_str().unwrap() }));
    let r2 = c.tool("vault_restore_asset", json!({ "id": "ItemConfig", "version": 1, "dest_dir": dest.to_str().unwrap() }));
    assert_ne!(r1["structuredContent"]["written"][0], r2["structuredContent"]["written"][0]);
    assert_eq!(std::fs::read_to_string(dest.join("ItemConfig.luau")).unwrap(), "return { Sword = 10 }");
    assert_eq!(c.tool("vault_restore_asset", json!({ "id": "ItemConfig", "dest_dir": "/tmp/../etc" }))["isError"], true);

    // memoria de proyecto (el proyecto lo crea el usuario en la app; aquí lo simulamos abriendo la bóveda)
    {
        let v = vault_core::Vault::open(home.path()).unwrap();
        v.create_project("Rift Tycoon", "tycoon").unwrap();
    }
    assert_eq!(c.tool("vault_list_projects", json!({}))["structuredContent"]["projects"][0]["name"], "Rift Tycoon");
    let logged = c.tool("vault_log_activity", json!({ "action": "decision", "detail": "Usar ProfileStore para datos", "project": "rift tycoon", "note_kind": "decision" }));
    assert_eq!(logged["isError"], false);
    let ctx = c.tool("vault_get_project_context", json!({ "project": "Rift Tycoon" }));
    assert_eq!(ctx["structuredContent"]["technical_decisions"][0]["text"], "Usar ProfileStore para datos");

    let deps = c.tool("vault_get_dependencies", json!({ "id": "ItemConfig" }));
    assert_eq!(deps["isError"], false);
    let unknown = c.rpc("resources/list", json!({}));
    assert_eq!(unknown["error"]["code"], -32601);
    drop(c);
    child.wait().unwrap();
}
