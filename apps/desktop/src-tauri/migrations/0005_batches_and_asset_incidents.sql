PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

ALTER TABLE products ADD COLUMN serial_number_policy TEXT NOT NULL DEFAULT 'optional'
  CHECK (serial_number_policy IN ('required', 'optional', 'not_applicable'));

UPDATE products
SET serial_number_policy = CASE WHEN tracking_type = 'serialized' THEN 'required' ELSE 'not_applicable' END;

CREATE TABLE movement_batches (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('entry', 'exit', 'transfer', 'adjustment')),
  reference TEXT NOT NULL DEFAULT '',
  note TEXT NOT NULL DEFAULT '',
  replenishment_request_id TEXT REFERENCES replenishment_requests(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  actor_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
  occurred_at TEXT NOT NULL
);

ALTER TABLE stock_movements ADD COLUMN batch_id TEXT
  REFERENCES movement_batches(id) ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE assets ADD COLUMN receipt_batch_id TEXT
  REFERENCES movement_batches(id) ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE replenishment_request_items ADD COLUMN received_quantity INTEGER NOT NULL DEFAULT 0
  CHECK (received_quantity >= 0 AND received_quantity <= purchase_quantity);

CREATE TABLE asset_incidents (
  id TEXT PRIMARY KEY,
  asset_id TEXT NOT NULL REFERENCES assets(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  reporter_id TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  custodian_name TEXT,
  description TEXT NOT NULL CHECK (length(trim(description)) >= 10),
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'internal_repair', 'external_assistance', 'warranty', 'resolved', 'disposed')),
  reviewer_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  resolution_note TEXT,
  reported_at TEXT NOT NULL,
  reviewed_at TEXT,
  resolved_at TEXT,
  version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1)
);

CREATE INDEX idx_movement_batches_date ON movement_batches(occurred_at DESC);
CREATE INDEX idx_stock_movements_batch ON stock_movements(batch_id);
CREATE INDEX idx_assets_receipt_batch ON assets(receipt_batch_id);
CREATE INDEX idx_asset_incidents_status_date ON asset_incidents(status, reported_at DESC);
CREATE INDEX idx_asset_incidents_asset_date ON asset_incidents(asset_id, reported_at DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

COMMIT;
