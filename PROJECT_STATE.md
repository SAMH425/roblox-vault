# ROBLOX VAULT — PROJECT_STATE

## Fase 1 — FUNDACIÓN ✅
- Tauri 2 + React 19 + TS estricto + Vite + Tailwind 4 + Motion + Lucide.
- `crates/vault-core`: SQLite (migraciones por `user_version`), originales en `blobs/` por SHA-256,
  duplicados, versiones sin sobrescribir, restauración sin sobrescribir, integridad, backups (VACUUM INTO),
  registro de actividad, anti path traversal. 3 pruebas pasan.
- App: Dashboard (datos reales), 3D Asset Library y Code Vault (importar, buscar, filtrar, estado, recuperar),
  Project Memory (crear/listar), Activity, Settings (integridad + backup).
- Bóveda en `Documentos/RobloxVault/data` (o variable `ROBLOX_VAULT_HOME`).

## Fase 2 — BIBLIOTECA ✅
- Systems Library: recursos con rol, instalación, configuración, compatibilidad, pruebas, estado; avisos si falta
  un original o hay recursos FAILED/DEPRECATED; recuperar sistema completo (aborta si falta una dependencia).
- Inspector: editar nombre/proyecto/metadatos, historial de versiones, nueva versión, recuperar versión, comparar (diff).
- Code Vault: tipo de script (convención Rojo .server/.client), búsqueda por contenido, resaltado Luau, copiar.
- Biblioteca: filtro por estado, orden, vista lista/cuadrícula.
- Actualizaciones automáticas: tauri-plugin-updater + GitHub Actions (`.github/workflows/release.yml`).
  Cada push a main → instalador firmado → la app instalada avisa y se actualiza.
- Pruebas: 4 de Rust + 2 de Vitest.

## Pendiente
- Fase 3: visor 3D, conversión RBXMX/RBXM, miniaturas.
- Fase 4: dependencias, historial, captura. Fase 5: MCP. Fase 6: plugin Studio. Fase 7: pulido + instalador.

## Limitaciones conocidas
- La app se compiló y probó en Linux; falta la primera ejecución en Windows.
