# ROBLOX VAULT

## Requisitos (Windows)
1. Node.js 20+  2. Rust (rustup.rs)  3. Visual Studio Build Tools con "Desarrollo de escritorio con C++"  4. WebView2 (ya viene en Windows 11)

## Ejecutar
```
cd app
npm install
npm run tauri dev
```
## Instalador
```
npm run tauri build
```
Sale en `app/src-tauri/target/release/bundle/`.

## Pruebas
```
cd crates/vault-core && cargo test
```

## Actualizaciones automáticas
1. Crea el repo en GitHub (público: el actualizador descarga de Releases sin login).
2. Secreto del repo `TAURI_SIGNING_PRIVATE_KEY` = contenido de `RobloxVault-keys/roblox-vault.key`. Nunca lo subas al repo.
3. Cada push a `main` compila el instalador y publica una Release. Instala la primera vez desde Releases; luego la app se actualiza sola.
