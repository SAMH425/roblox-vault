use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use vault_core::*;

struct AppState(Mutex<Vault>);
type R<T> = std::result::Result<T, String>;
fn e<E: std::fmt::Display>(x: E) -> String { x.to_string() }

#[tauri::command] fn get_stats(s: tauri::State<AppState>) -> R<Stats> { s.0.lock().map_err(e)?.stats().map_err(e) }
#[tauri::command] fn list_projects(s: tauri::State<AppState>) -> R<Vec<Project>> { s.0.lock().map_err(e)?.list_projects().map_err(e) }
#[tauri::command] fn create_project(s: tauri::State<AppState>, name: String, description: String) -> R<Project> { s.0.lock().map_err(e)?.create_project(&name, &description).map_err(e) }
#[tauri::command] fn list_assets(s: tauri::State<AppState>, kind: Option<String>, project_id: Option<String>, query: Option<String>) -> R<Vec<Asset>> {
    s.0.lock().map_err(e)?.list_assets(kind.as_deref(), project_id.as_deref(), query.as_deref().filter(|q| !q.is_empty())).map_err(e) }
#[tauri::command] fn import_files(s: tauri::State<AppState>, paths: Vec<String>, project_id: Option<String>) -> R<Vec<ImportResult>> {
    let v = s.0.lock().map_err(e)?;
    paths.iter().map(|p| v.import_file(&PathBuf::from(p), project_id.as_deref(), None).map_err(e)).collect() }
#[tauri::command] fn set_status(s: tauri::State<AppState>, id: String, status: String) -> R<()> { s.0.lock().map_err(e)?.set_status(&id, &status).map_err(e) }
#[tauri::command] fn restore_asset(s: tauri::State<AppState>, id: String, version: Option<i64>, dest_dir: String) -> R<String> {
    s.0.lock().map_err(e)?.restore_asset(&id, version, &PathBuf::from(dest_dir)).map(|p| p.display().to_string()).map_err(e) }
#[tauri::command] fn verify_integrity(s: tauri::State<AppState>) -> R<IntegrityReport> { s.0.lock().map_err(e)?.verify_integrity().map_err(e) }
#[tauri::command] fn create_backup(s: tauri::State<AppState>) -> R<String> { s.0.lock().map_err(e)?.backup().map(|p| p.display().to_string()).map_err(e) }
#[tauri::command] fn recent_activity(s: tauri::State<AppState>, limit: i64) -> R<Vec<Activity>> { s.0.lock().map_err(e)?.recent_activity(limit).map_err(e) }
#[tauri::command] fn vault_path(s: tauri::State<AppState>) -> R<String> { Ok(s.0.lock().map_err(e)?.root.display().to_string()) }

#[tauri::command] fn list_versions(s: tauri::State<AppState>, id: String) -> R<Vec<Version>> { s.0.lock().map_err(e)?.list_versions(&id).map_err(e) }
#[tauri::command] fn get_text(s: tauri::State<AppState>, id: String, version: Option<i64>) -> R<Option<String>> { s.0.lock().map_err(e)?.get_text(&id, version).map_err(e) }
#[tauri::command] fn add_version(s: tauri::State<AppState>, id: String, path: String) -> R<ImportResult> { s.0.lock().map_err(e)?.import_file(&PathBuf::from(path), None, Some(&id)).map_err(e) }
#[tauri::command] fn update_asset(s: tauri::State<AppState>, id: String, name: Option<String>, project_id: Option<String>, metadata: Option<serde_json::Value>) -> R<Asset> {
    let pid = project_id.as_deref().map(|p| if p.is_empty() { None } else { Some(p) });
    s.0.lock().map_err(e)?.update_asset(&id, name.as_deref(), pid, metadata.as_ref()).map_err(e) }
#[tauri::command] fn list_systems(s: tauri::State<AppState>, project_id: Option<String>, query: Option<String>) -> R<Vec<System>> { s.0.lock().map_err(e)?.list_systems(project_id.as_deref(), query.as_deref()).map_err(e) }
#[tauri::command] fn create_system(s: tauri::State<AppState>, input: SystemInput) -> R<System> { s.0.lock().map_err(e)?.create_system(&input).map_err(e) }
#[tauri::command] fn update_system(s: tauri::State<AppState>, id: String, input: SystemInput) -> R<System> { s.0.lock().map_err(e)?.update_system(&id, &input).map_err(e) }
#[tauri::command] fn set_system_status(s: tauri::State<AppState>, id: String, status: String) -> R<()> { s.0.lock().map_err(e)?.set_system_status(&id, &status).map_err(e) }
#[tauri::command] fn add_system_member(s: tauri::State<AppState>, system_id: String, asset_id: String, role: String) -> R<()> { s.0.lock().map_err(e)?.add_system_member(&system_id, &asset_id, &role).map_err(e) }
#[tauri::command] fn remove_system_member(s: tauri::State<AppState>, system_id: String, asset_id: String) -> R<()> { s.0.lock().map_err(e)?.remove_system_member(&system_id, &asset_id).map_err(e) }
#[tauri::command] fn restore_system(s: tauri::State<AppState>, id: String, dest_dir: String) -> R<Vec<String>> {
    s.0.lock().map_err(e)?.restore_system(&id, &PathBuf::from(dest_dir)).map(|v| v.iter().map(|p| p.display().to_string()).collect()).map_err(e) }

#[tauri::command] fn read_blob(s: tauri::State<AppState>, id: String, version: Option<i64>) -> R<tauri::ipc::Response> {
    s.0.lock().map_err(e)?.read_blob(&id, version).map(tauri::ipc::Response::new).map_err(e) }
#[tauri::command] fn roblox_scene(s: tauri::State<AppState>, id: String, version: Option<i64>) -> R<roblox::RbxModel> { s.0.lock().map_err(e)?.roblox_scene(&id, version).map_err(e) }
#[tauri::command] fn save_thumbnail(s: tauri::State<AppState>, id: String, png: Vec<u8>) -> R<()> { s.0.lock().map_err(e)?.save_thumbnail(&id, &png).map_err(e) }
#[tauri::command] fn thumbnail(s: tauri::State<AppState>, id: String) -> R<tauri::ipc::Response> { s.0.lock().map_err(e)?.thumbnail(&id).map(|o| tauri::ipc::Response::new(o.unwrap_or_default())).map_err(e) }

#[tauri::command] fn add_dependency(s: tauri::State<AppState>, asset_id: String, depends_on: String, note: String) -> R<()> { s.0.lock().map_err(e)?.add_dependency(&asset_id, &depends_on, &note).map_err(e) }
#[tauri::command] fn remove_dependency(s: tauri::State<AppState>, asset_id: String, depends_on: String) -> R<()> { s.0.lock().map_err(e)?.remove_dependency(&asset_id, &depends_on).map_err(e) }
#[tauri::command] fn dependency_report(s: tauri::State<AppState>, id: String) -> R<DepReport> { s.0.lock().map_err(e)?.dependency_report(&id).map_err(e) }
#[tauri::command] fn asset_history(s: tauri::State<AppState>, id: String) -> R<Vec<Activity>> { s.0.lock().map_err(e)?.asset_history(&id, 100).map_err(e) }
#[tauri::command] fn list_captures(s: tauri::State<AppState>, state: Option<String>) -> R<Vec<Capture>> { s.0.lock().map_err(e)?.list_captures(state.as_deref()).map_err(e) }
#[tauri::command] fn decide_capture(s: tauri::State<AppState>, id: String, approve: bool) -> R<Capture> { s.0.lock().map_err(e)?.decide_capture(&id, approve, "user").map_err(e) }
#[tauri::command] fn get_setting(s: tauri::State<AppState>, key: String) -> R<Option<String>> { s.0.lock().map_err(e)?.setting(&key).map_err(e) }
#[tauri::command] fn set_setting(s: tauri::State<AppState>, key: String, value: String) -> R<()> { s.0.lock().map_err(e)?.set_setting(&key, &value).map_err(e) }
#[tauri::command] fn project_context(s: tauri::State<AppState>, id: String) -> R<ProjectContext> { s.0.lock().map_err(e)?.project_context(&id).map_err(e) }
#[tauri::command] fn update_project(s: tauri::State<AppState>, id: String, name: String, description: String, architecture: String) -> R<Project> { s.0.lock().map_err(e)?.update_project(&id, &name, &description, &architecture).map_err(e) }
#[tauri::command] fn add_project_note(s: tauri::State<AppState>, project_id: String, kind: String, text: String) -> R<ProjectNote> { s.0.lock().map_err(e)?.add_project_note(&project_id, &kind, &text).map_err(e) }
#[tauri::command] fn delete_project_note(s: tauri::State<AppState>, id: i64) -> R<()> { s.0.lock().map_err(e)?.delete_project_note(id).map_err(e) }

#[derive(serde::Serialize)]
struct McpInfo { path: String, exists: bool, vault_root: String, claude_code: String, claude_desktop: serde_json::Value }

/// Ruta del servidor MCP instalado junto a la app y comandos listos para copiar.
#[tauri::command] fn mcp_info(s: tauri::State<AppState>) -> R<McpInfo> {
    let root = s.0.lock().map_err(e)?.root.display().to_string();
    let exe = std::env::current_exe().map_err(e)?;
    let name = if cfg!(windows) { "vault-mcp.exe" } else { "vault-mcp" };
    let path = exe.parent().map(|d| d.join(name)).unwrap_or_default();
    let p = path.display().to_string();
    Ok(McpInfo {
        exists: path.exists(),
        claude_code: format!("claude mcp add roblox-vault -s user -e ROBLOX_VAULT_HOME=\"{root}\" -- \"{p}\""),
        claude_desktop: serde_json::json!({ "mcpServers": { "roblox-vault": { "command": p, "env": { "ROBLOX_VAULT_HOME": root } } } }),
        path: p, vault_root: root,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // ROBLOX_VAULT_HOME permite compartir la bóveda con el servidor MCP.
            let root = std::env::var("ROBLOX_VAULT_HOME").map(PathBuf::from)
                .unwrap_or(app.path().document_dir()?.join("RobloxVault").join("data"));
            app.manage(AppState(Mutex::new(Vault::open(root)?)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_stats, list_projects, create_project, list_assets, import_files, set_status, restore_asset, verify_integrity, create_backup, recent_activity, vault_path, list_versions, get_text, add_version, update_asset, list_systems, create_system, update_system, set_system_status, add_system_member, remove_system_member, restore_system, read_blob, roblox_scene, save_thumbnail, thumbnail, add_dependency, remove_dependency, dependency_report, asset_history, list_captures, decide_capture, get_setting, set_setting, project_context, update_project, add_project_note, delete_project_note, mcp_info])
        .run(tauri::generate_context!())
        .expect("error al iniciar ROBLOX VAULT");
}
