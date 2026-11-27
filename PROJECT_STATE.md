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

## Fase 5 — MCP ✅
- `crates/vault-mcp`: servidor MCP propio (stdio, JSON-RPC 2.0, protocolo 2025-06-18) sobre vault-core.
  Se compila en CI y se instala junto a la app (`vault-mcp.exe`). Settings → "Conectar Claude Code" da el comando.
- 12 herramientas: vault_search, vault_get_asset, vault_save_asset, vault_get_script, vault_get_system,
  vault_restore_asset, vault_update_asset, vault_get_dependencies, vault_validate_asset, vault_list_projects,
  vault_get_project_context, vault_log_activity. Validación de entrada, JSON estructurado, errores controlados
  (isError), todo queda en Activity con actor "claude". Acepta id o nombre.
- Protocolo para Claude en `instructions` (consultar antes de crear, reutilizar, versionar, registrar pruebas reales).
- Validación real: SHA-256 + sintaxis Luau (full-moon, dialecto Luau) + RBXM/GLB/JSON + scripts internos + dependencias.
  Sin pruebas → máximo PARTIALLY VERIFIED; VERIFIED solo con pruebas superadas; cualquier fallo → FAILED.
  Claude no puede poner VERIFIED a mano.
- Búsqueda: exacta por nombre y por contenido. La búsqueda semántica local opcional queda pendiente (Fase 7).
- Pruebas: 10 Rust (núcleo) + 1 prueba de punta a punta que lanza el binario y habla MCP + 5 Vitest.

## Fase 6 — ROBLOX STUDIO ✅ (pendiente de prueba en tu Studio)
- `plugin/RobloxVault.plugin.luau`: pestaña ROBLOX VAULT con panel acoplable. Conectar, vincular el juego por
  PlaceId a un proyecto, buscar (todo o solo el proyecto), insertar modelos/scripts (con confirmación),
  enviar la selección (modelos vía SerializationService → .rbxm; scripts con su Source y clase), nueva versión
  automática si la instancia ya vino de la bóveda (atributo VaultAssetId), registrar cambios, importar archivo
  (.rbxm con StudioService:PromptImportFileAsync) como alternativa si no hay conexión.
- Seguridad: solo 127.0.0.1 + token por instalación; nunca ejecuta código: scripts no VERIFIED se insertan
  desactivados, scripts con errores de sintaxis no se insertan; todo dentro de ChangeHistoryService (deshacer).
- APIs verificadas en la documentación oficial: SerializationService, EncodingService.Base64Encode/Decode,
  HttpService a localhost desde plugins.
- `crates/vault-bridge`: servidor HTTP local en la app (puerto 47515). Convierte .rbxmx → .rbxm para Studio.
  Settings → "Instalar plugin en Studio" lo escribe en %LOCALAPPDATA%\Roblox\Plugins con el token.
- Pruebas: 3 Rust (rutas + servidor HTTP real + el plugin se valida como Luau con full-moon).

## Fase 7 — PULIDO (en curso)
- Búsqueda por relevancia local (BM25, camelCase, prefijos, código) en la app, MCP y plugin. Sin IA externa.
- Documentación: README, docs/ARCHITECTURE, docs/MCP, docs/PLUGIN, docs/LIMITATIONS.
- Instalador Windows (NSIS) + actualizaciones automáticas firmadas ya funcionando.

## Pendiente
Fase 7: pulido + instalador.

## Limitaciones conocidas
- La app se compiló y probó en Linux; falta la primera ejecución en Windows.
