-- Initial schema for GuildForge state store.
-- See ADR-0002 for the design.

-- Schema metadata: holds the current schema version and other globals.
CREATE TABLE IF NOT EXISTS schema_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- The authoritative record of what GuildForge last applied.
-- `data` is the JSON serialization of the Resource enum variant.
-- `content_hash` is blake3 of `data`, used for fast diffing.
-- `tainted` marks resources whose last apply failed (will be recreated).
CREATE TABLE IF NOT EXISTS resources (
    addr          TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    provider      TEXT NOT NULL,
    data          TEXT NOT NULL,
    content_hash  TEXT NOT NULL,
    tainted       INTEGER NOT NULL DEFAULT 0,
    updated_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_resources_kind ON resources(kind);

-- Audit log of every apply / destroy / doctor run.
CREATE TABLE IF NOT EXISTS migrations_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    applied_at  TEXT NOT NULL,
    plan_hash   TEXT NOT NULL,
    summary     TEXT NOT NULL
);

-- Snapshots of live state taken by `guildforge doctor` for drift
-- detection. Old snapshots are pruned automatically.
CREATE TABLE IF NOT EXISTS drift_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    taken_at    TEXT NOT NULL,
    snapshot    TEXT NOT NULL
);

-- Insert schema version marker.
INSERT OR IGNORE INTO schema_meta (key, value) VALUES ('schema_version', '1');
