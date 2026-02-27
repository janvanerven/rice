-- Rename cover_image_url to cover_image_path for consistency with trips table.
-- SQLite doesn't support ALTER COLUMN RENAME, so we recreate the table.
CREATE TABLE accommodations_new (
    id TEXT PRIMARY KEY NOT NULL,
    trip_id TEXT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    address TEXT,
    check_in TEXT,
    check_out TEXT,
    notes TEXT,
    cover_image_path TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

INSERT INTO accommodations_new (id, trip_id, name, address, check_in, check_out, notes, cover_image_path, created_at, updated_at)
SELECT id, trip_id, name, address, check_in, check_out, notes, cover_image_url, created_at, updated_at
FROM accommodations;

DROP TABLE accommodations;
ALTER TABLE accommodations_new RENAME TO accommodations;

-- Image attributions for auto-fetched cover images
CREATE TABLE image_attributions (
    entity_type TEXT NOT NULL CHECK (entity_type IN ('trip', 'accommodation', 'activity')),
    entity_id   TEXT NOT NULL,
    author_name TEXT NOT NULL,
    author_url  TEXT NOT NULL,
    source_url  TEXT NOT NULL,
    fetched_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (entity_type, entity_id)
);
