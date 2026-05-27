ALTER TABLE
    Records
ADD
    COLUMN IF NOT EXISTS replay_available BOOLEAN NOT NULL DEFAULT FALSE
AFTER
    plugin_version_id;
