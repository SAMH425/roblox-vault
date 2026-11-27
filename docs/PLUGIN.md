# Plugin de Roblox Studio

Lo instala la app en `%LOCALAPPDATA%\Roblox\Plugins\RobloxVault.lua` con el puerto y un token propio.

- **Vincular este juego**: asocia el PlaceId a un proyecto (el lugar debe estar publicado).
- **Buscar / Insertar**: siempre pide confirmación. Scripts de recursos no VERIFIED se insertan desactivados;
  scripts con errores de sintaxis no se insertan. Todo se puede deshacer (Ctrl+Z).
- **Enviar selección**: modelos con `SerializationService:SerializeInstancesAsync` (.rbxm), scripts con su código y clase.
  Si la instancia vino de la bóveda (atributo `VaultAssetId`) se guarda como versión nueva.
- **Importar archivo**: alternativa sin conexión: en la app usa *Recuperar* y aquí elige el `.rbxm`.
- **Registrar cambio**: añade una nota al historial.

La primera vez Studio pide permiso para conectarse a `127.0.0.1`: acéptalo.

APIs usadas (documentación oficial de Roblox): HttpService:RequestAsync, SerializationService,
EncodingService:Base64Encode/Base64Decode, ChangeHistoryService, Selection, StudioService:PromptImportFileAsync,
plugin:CreateDockWidgetPluginGui.
