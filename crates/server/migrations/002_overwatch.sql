CREATE TABLE IF NOT EXISTS overwatch (
    host_id TEXT NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    container_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (host_id, container_id)
);

CREATE INDEX IF NOT EXISTS idx_overwatch_enabled ON overwatch(enabled);
