-- Role column (SQLite: add with default, then backfill masters)
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'member';

UPDATE users
SET role = 'master'
WHERE username = 'master'
   OR id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1);

-- Ensure at least one master if users exist but none matched above oddly
UPDATE users
SET role = 'master'
WHERE id = (
    SELECT id FROM users
    WHERE NOT EXISTS (SELECT 1 FROM users WHERE role = 'master')
    ORDER BY created_at ASC
    LIMIT 1
);

CREATE TABLE IF NOT EXISTS activity_log (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT,
    resource_id TEXT,
    detail TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_activity_log_created_at ON activity_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_activity_log_user_id ON activity_log(user_id);
