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

## Fase 3 — VISOR 3D ✅
- Pipeline Roblox real: `.rbxm` (binario) y `.rbxmx` (XML) con rbx_binary/rbx_xml (los lectores de Rojo).
  Extrae jerarquía, piezas (tamaño, CFrame, color, material, forma), scripts con código. Metadatos al importar:
  piezas, instancias, dimensiones en studs, materiales, scripts.
- Visor (Three.js + R3F + Drei): GLB, GLTF, OBJ, FBX, RBXM, RBXMX. Órbita, zoom, paneo, ajustar a pantalla,
  reiniciar cámara, luz regulable, entorno local (sin internet), alambre/materiales, dimensiones, piezas,
  triángulos, jerarquía, pantalla completa.
- Miniaturas reales: se renderizan al abrir el modelo y se guardan en `thumbs/`. Botón para regenerar.
- Pruebas: 6 Rust + 5 Vitest. Verificado con captura en navegador.
- Limitaciones: MeshPart/Unions guardan la malla en la nube de Roblox → se dibujan como caja en alambre con su
  tamaño real. GLTF con .bin externos → importar como GLB.

## Fase 4 — MEMORIA ✅
- Dependencias entre recursos: directas, en cadena, quién lo usa; detecta FAILED/DEPRECATED, originales
  faltantes y ciclos. Lee `require(...)` del código Luau y sugiere módulos de la bóveda o avisa si no existen.
- Historial por recurso (actividad ligada al recurso).
- Captura híbrida: probado+OK → PARTIALLY VERIFIED automático (configurable); probado+falla → FAILED;
  sin probar → cola de aprobación (Activity → Captura). Guarda la salida de las pruebas en metadatos.
- Project Memory: arquitectura, decisiones técnicas, errores conocidos, cambios recientes, recursos,
  sistemas y actividad del proyecto.
- Animations: KeyframeSequence en .rbxm/.rbxmx → keyframes, duración, bucle, articulaciones.
- Pruebas: 9 Rust + 5 Vitest.

## Pendiente
- Fase 5: MCP. Fase 6: plugin Studio. Fase 7: pulido + instalador.

## Limitaciones conocidas
- La app se compiló y probó en Linux; falta la primera ejecución en Windows.
