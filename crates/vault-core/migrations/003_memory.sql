ALTER TABLE activity ADD COLUMN asset_id TEXT;
CREATE INDEX IF NOT EXISTS idx_activity_asset ON activity(asset_id);
CREATE TABLE IF NOT EXISTS asset_deps (
  asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
  depends_on TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
  note TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (asset_id, depends_on), CHECK (asset_id <> depends_on));
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS captures (
  id TEXT PRIMARY KEY, source_path TEXT NOT NULL, project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  tested INTEGER NOT NULL, test_passed INTEGER, test_output TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '',
  actor TEXT NOT NULL, state TEXT NOT NULL CHECK (state IN ('pending','approved','rejected')), asset_id TEXT,
  created_at TEXT NOT NULL, decided_at TEXT);
CREATE TABLE IF NOT EXISTS project_notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('decision','bug','change','architecture')), text TEXT NOT NULL, created_at TEXT NOT NULL);
