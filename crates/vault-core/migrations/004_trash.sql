ALTER TABLE assets ADD COLUMN deleted_at TEXT;
CREATE INDEX IF NOT EXISTS idx_assets_deleted ON assets(deleted_at);
