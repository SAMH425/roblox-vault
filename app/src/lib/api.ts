import { invoke } from '@tauri-apps/api/core'

export type Status = 'VERIFIED' | 'PARTIALLY VERIFIED' | 'PENDING' | 'FAILED' | 'DEPRECATED'
export const STATUSES: Status[] = ['VERIFIED', 'PARTIALLY VERIFIED', 'PENDING', 'FAILED', 'DEPRECATED']

export interface Project { id: string; name: string; description: string; architecture: string; created_at: string; updated_at: string }
export interface Asset {
  id: string; project_id: string | null; project_name: string | null; kind: string; name: string; format: string
  original_name: string; status: Status; metadata: Record<string, unknown>; version: number; sha256: string; size: number
  created_at: string; updated_at: string
}
export interface ImportResult { asset: Asset; duplicate_of: string | null; new_version: boolean }
export interface Stats { projects: number; assets: number; by_kind: [string, number][]; by_status: [string, number][]; storage_bytes: number; activity_24h: number }
export interface Activity { id: number; at: string; actor: string; action: string; target: string; detail: string; destructive: boolean }
export interface IntegrityReport { checked: number; ok: number; missing: string[]; corrupted: string[] }

export interface Version { version: number; sha256: string; size: number; note: string; created_at: string }
export interface SystemInput { name: string; project_id: string | null; description: string; install: string; config: string; compatibility: string; tests: string }
export interface VaultSystem extends SystemInput { id: string; status: Status; created_at: string; updated_at: string; members: { role: string; asset: Asset }[]; warnings: string[] }

export interface DepNode { asset: Asset; note: string; depth: number }
export interface DepReport { direct: DepNode[]; all: DepNode[]; dependents: Asset[]; problems: string[]; unresolved_requires: string[]; suggestions: Asset[] }
export interface Capture { id: string; source_path: string; project_id: string | null; tested: boolean; test_passed: boolean | null; test_output: string; note: string; actor: string; state: 'pending' | 'approved' | 'rejected'; asset_id: string | null; created_at: string }
export type NoteKind = 'decision' | 'bug' | 'change' | 'architecture'
export interface ProjectNote { id: number; kind: NoteKind; text: string; created_at: string }
export interface ProjectContext { project: Project; assets: Asset[]; systems: VaultSystem[]; notes: ProjectNote[]; recent: Activity[] }

export interface StyleGuide { art_style: string; palette: string[]; materials: string[]; font: string; icon_pack: string; icon_variant: string; ui_notes: string; do_not: string }

export const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new Error('Abre la app con "npm run tauri dev": el navegador no tiene acceso a la bóveda.')
  return invoke<T>(cmd, args)
}

export const api = {
  stats: () => call<Stats>('get_stats'),
  projects: () => call<Project[]>('list_projects'),
  createProject: (name: string, description: string) => call<Project>('create_project', { name, description }),
  assets: (f: { kind?: string; projectId?: string; query?: string } = {}) =>
    call<Asset[]>('list_assets', { kind: f.kind ?? null, projectId: f.projectId ?? null, query: f.query ?? null }),
  importFiles: (paths: string[], projectId?: string) => call<ImportResult[]>('import_files', { paths, projectId: projectId ?? null }),
  setStatus: (id: string, status: Status) => call<void>('set_status', { id, status }),
  restore: (id: string, destDir: string, version?: number) => call<string>('restore_asset', { id, version: version ?? null, destDir }),
  verify: () => call<IntegrityReport>('verify_integrity'),
  backup: () => call<string>('create_backup'),
  activity: (limit = 100) => call<Activity[]>('recent_activity', { limit }),
  vaultPath: () => call<string>('vault_path'),
  blob: (id: string, version?: number) => call<ArrayBuffer>('read_blob', { id, version: version ?? null }),
  scene: (id: string, version?: number) => call<import('./models').RbxModel>('roblox_scene', { id, version: version ?? null }),
  saveThumb: (id: string, png: Uint8Array) => call<void>('save_thumbnail', { id, png: Array.from(png) }),
  thumb: async (id: string) => { const b = await call<ArrayBuffer>('thumbnail', { id }); return b.byteLength ? b : null },
  addDep: (assetId: string, dependsOn: string, note = '') => call<void>('add_dependency', { assetId, dependsOn, note }),
  removeDep: (assetId: string, dependsOn: string) => call<void>('remove_dependency', { assetId, dependsOn }),
  deps: (id: string) => call<DepReport>('dependency_report', { id }),
  history: (id: string) => call<Activity[]>('asset_history', { id }),
  captures: (state?: string) => call<Capture[]>('list_captures', { state: state ?? null }),
  decideCapture: (id: string, approve: boolean) => call<Capture>('decide_capture', { id, approve }),
  getSetting: (key: string) => call<string | null>('get_setting', { key }),
  setSetting: (key: string, value: string) => call<void>('set_setting', { key, value }),
  projectContext: (id: string) => call<ProjectContext>('project_context', { id }),
  updateProject: (id: string, name: string, description: string, architecture: string) => call<Project>('update_project', { id, name, description, architecture }),
  addNote: (projectId: string, kind: NoteKind, text: string) => call<ProjectNote>('add_project_note', { projectId, kind, text }),
  deleteNote: (id: number) => call<void>('delete_project_note', { id }),
  mcpInfo: () => call<{ path: string; exists: boolean; vault_root: string; claude_code: string; claude_desktop: unknown }>('mcp_info'),
  bridgeInfo: () => call<{ running: boolean; port: number; error: string | null; plugin_path: string | null; plugin_installed: boolean }>('bridge_info'),
  installPlugin: () => call<string>('install_plugin'),
  ranked: (query: string, f: { kind?: string; projectId?: string } = {}) => call<{ asset: Asset; score: number; matched: string[] }[]>('search_ranked', { query, kind: f.kind ?? null, projectId: f.projectId ?? null }),
  importFolder: (path: string, pack: string, projectId?: string) => call<{ imported: number; duplicates: number; skipped: number; errors: string[] }>('import_folder', { path, pack, projectId: projectId ?? null }),
  installTemplates: () => call<{ systems: { system: string; added: number; updated: number; members: number }[] }>('install_templates'),
  trash: (ids: string[]) => call<{ moved: number; warnings: string[] }>('trash_assets', { ids }),
  restoreTrash: (ids: string[]) => call<number>('restore_trash', { ids }),
  listTrash: () => call<{ asset: Asset; deleted_at: string }[]>('list_trash'),
  emptyTrash: (ids?: string[]) => call<{ removed: number; files: number; backup: string }>('empty_trash', { ids: ids ?? null }),
  getStyle: (projectId: string) => call<StyleGuide>('get_style', { projectId }),
  setStyle: (projectId: string, guide: StyleGuide) => call<StyleGuide>('set_style', { projectId, guide }),
  setRobloxId: (id: string, robloxId: string) => call<Asset>('set_roblox_id', { id, robloxId }),
  saveVersionThumb: (id: string, version: number, png: Uint8Array) => call<void>('save_version_thumbnail', { id, version, png: Array.from(png) }),
  versionThumb: async (id: string, version: number) => { const b = await call<ArrayBuffer>('version_thumbnail', { id, version }); return b.byteLength ? b : null },
  missingThumbs: () => call<Asset[]>('missing_thumbnails'),
  versions: (id: string) => call<Version[]>('list_versions', { id }),
  text: (id: string, version?: number) => call<string | null>('get_text', { id, version: version ?? null }),
  addVersion: (id: string, path: string) => call<ImportResult>('add_version', { id, path }),
  updateAsset: (id: string, p: { name?: string; projectId?: string; metadata?: Record<string, unknown> }) =>
    call<Asset>('update_asset', { id, name: p.name ?? null, projectId: p.projectId ?? null, metadata: p.metadata ?? null }),
  systems: (f: { projectId?: string; query?: string } = {}) => call<VaultSystem[]>('list_systems', { projectId: f.projectId ?? null, query: f.query ?? null }),
  createSystem: (input: SystemInput) => call<VaultSystem>('create_system', { input }),
  updateSystem: (id: string, input: SystemInput) => call<VaultSystem>('update_system', { id, input }),
  setSystemStatus: (id: string, status: Status) => call<void>('set_system_status', { id, status }),
  addMember: (systemId: string, assetId: string, role: string) => call<void>('add_system_member', { systemId, assetId, role }),
  removeMember: (systemId: string, assetId: string) => call<void>('remove_system_member', { systemId, assetId }),
  restoreSystem: (id: string, destDir: string) => call<string[]>('restore_system', { id, destDir }),
}

export function formatBytes(n: number) {
  const u = ['B', 'KB', 'MB', 'GB']; let i = 0
  while (n >= 1024 && i < u.length - 1) { n /= 1024; i++ }
  return `${n.toFixed(i ? 1 : 0)} ${u[i]}`
}
