# Arquitectura

```
Roblox Studio ──plugin Luau──► 127.0.0.1:47515 (vault-bridge) ─┐
Claude Code ──stdio MCP──────► vault-mcp.exe ──────────────────┼──► vault-core ──► SQLite (vault.db, WAL)
App (React) ──Tauri invoke───► comandos Rust (app/src-tauri) ──┘                └─► blobs/aa/<sha256> (originales)
```

## Bóveda en disco (`Documentos/RobloxVault/data` o `ROBLOX_VAULT_HOME`)
- `vault.db`: metadatos. Migraciones por `PRAGMA user_version` (`crates/vault-core/migrations`).
- `blobs/<2 primeros>/<sha256>`: originales inmutables, direccionados por contenido. Nunca se modifican ni sobrescriben.
- `thumbs/<id>.png`: miniaturas renderizadas por el visor.
- `backups/`: copias consistentes de la base de datos (`VACUUM INTO`).
- `incoming/`: temporal del puente con Studio.

## Reglas del núcleo
- Importar copia el archivo, verifica el hash de la copia y no toca el original del usuario.
- Mismo SHA-256 = duplicado (no se guarda dos veces). Una versión nueva vuelve el estado a PENDING.
- Restaurar nunca sobrescribe (añade sufijo) y aborta si el original almacenado está corrupto.
- Rutas validadas contra path traversal. Todas las operaciones quedan en `activity` (las destructivas marcadas).

## Estados de validación
| Estado | Cuándo |
|---|---|
| PENDING | recién importado o con versión nueva |
| PARTIALLY VERIFIED | estructura correcta pero sin pruebas reales (o con `require` sin resolver) |
| VERIFIED | comprobaciones correctas **y** pruebas reales superadas |
| FAILED | integridad, sintaxis, dependencias o pruebas fallan |
| DEPRECATED | lo marca el usuario o Claude; la validación no lo cambia |

Integridad (el archivo no cambió) ≠ funcionamiento (lo prueban las pruebas reales).
