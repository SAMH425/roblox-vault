# Servidor MCP

Binario `vault-mcp.exe`, instalado junto a la app. Conexión (Settings → Conectar Claude Code):
```
claude mcp add roblox-vault -s user -e ROBLOX_VAULT_HOME="<bóveda>" -- "<ruta>\vault-mcp.exe"
```

| Herramienta | Para qué |
|---|---|
| vault_search | Buscar por relevancia (camelCase, prefijos, código) o texto exacto; incluye sistemas |
| vault_get_asset | Ficha, versiones, dependencias y quién lo usa |
| vault_save_asset | Registrar lo construido con el resultado real de pruebas (captura híbrida) |
| vault_get_script | Código de un script (cualquier versión) |
| vault_get_system | Sistema completo, opcionalmente con el código |
| vault_restore_asset | Copiar a una carpeta sin sobrescribir; opcionalmente con dependencias |
| vault_update_asset | Versión nueva, nombre, proyecto, metadatos; estado solo DEPRECATED/PENDING |
| vault_get_dependencies | Árbol, dependientes, problemas, `require` sin resolver; añadir dependencias |
| vault_validate_asset | Integridad + sintaxis Luau + RBXM/GLB/JSON + dependencias + pruebas |
| vault_list_projects | Proyectos con recuentos |
| vault_get_project_context | Arquitectura, decisiones, errores conocidos, cambios, recursos, sistemas |
| vault_log_activity | Registrar decisiones, errores, cambios o actividad |

Las respuestas llevan `structuredContent`; los errores usan `isError: true` y quedan registrados.
El servidor envía en `instructions` el protocolo que Claude debe seguir (consultar → reutilizar → versionar → registrar → probar → validar).
