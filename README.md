# ROBLOX VAULT

Memoria técnica permanente para tus juegos de Roblox: guarda, organiza, visualiza, valida y recupera
modelos, scripts, sistemas, animaciones y decisiones técnicas. Claude Code la consulta por MCP y
Roblox Studio se conecta con un plugin.

## Instalar
Descarga el `.exe` de la última versión en **Releases** e instálalo. Las siguientes versiones llegan solas:
la app muestra **Actualizar y reiniciar**.

Después, en **Settings**:
1. **Instalar plugin en Studio** → reinicia Studio → pestaña *ROBLOX VAULT*.
2. **Conectar Claude Code** → copia el comando y pégalo una vez en PowerShell.

## Qué hay dentro
| Carpeta | Qué es |
|---|---|
| `app/` | App de escritorio (Tauri 2 + React + TypeScript + Tailwind + Motion + Three.js) |
| `crates/vault-core` | Núcleo: SQLite, almacenamiento por SHA-256, versiones, dependencias, validación, lector RBXM/RBXMX |
| `crates/vault-mcp` | Servidor MCP para Claude Code (12 herramientas) |
| `crates/vault-bridge` | Puente local HTTP (127.0.0.1) para el plugin de Studio |
| `plugin/` | Plugin de Roblox Studio en Luau |
| `docs/` | Arquitectura, MCP, plugin, limitaciones |

## Desarrollo
```
cd app && npm install && npm run tauri dev
cargo test --manifest-path crates/vault-core/Cargo.toml
cargo test --manifest-path crates/vault-mcp/Cargo.toml
cargo test --manifest-path crates/vault-bridge/Cargo.toml
cd app && npx vitest run
```
Requisitos en Windows: Node 20+, Rust, Visual Studio Build Tools (C++), WebView2.

## Publicar una versión
Cada push a `main` ejecuta las pruebas, compila el instalador firmado y publica una Release
(`.github/workflows/release.yml`). El secreto `TAURI_SIGNING_PRIVATE_KEY` firma las actualizaciones;
la clave pública está en `app/src-tauri/tauri.conf.json`. Si pierdes la clave privada, las apps instaladas
no podrán actualizarse y habrá que reinstalar una vez.

Estado detallado: [PROJECT_STATE.md](PROJECT_STATE.md).
