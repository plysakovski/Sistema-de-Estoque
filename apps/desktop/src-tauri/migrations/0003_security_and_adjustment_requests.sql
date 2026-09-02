PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

ALTER TABLE users ADD COLUMN password_hash TEXT;
ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_login_attempts >= 0);
ALTER TABLE users ADD COLUMN locked_until TEXT;
ALTER TABLE users ADD COLUMN password_changed_at TEXT;
ALTER TABLE users ADD COLUMN last_login_at TEXT;
ALTER TABLE users ADD COLUMN session_version INTEGER NOT NULL DEFAULT 1 CHECK (session_version >= 1);

CREATE TABLE adjustment_requests (
  id TEXT PRIMARY KEY,
  requester_id TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  asset_id TEXT REFERENCES assets(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  kind TEXT NOT NULL CHECK (kind IN ('quantity_adjustment', 'asset_update')),
  location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  requested_quantity INTEGER CHECK (requested_quantity >= 0),
  requested_status TEXT CHECK (requested_status IN ('available', 'in_use', 'maintenance', 'disposed')),
  requested_location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  description TEXT NOT NULL CHECK (length(trim(description)) >= 10),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'cancelled')),
  reviewer_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  review_note TEXT,
  before_data TEXT NOT NULL,
  target_version INTEGER NOT NULL CHECK (target_version >= 1),
  requested_at TEXT NOT NULL,
  reviewed_at TEXT,
  version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
  CHECK (
    (kind = 'quantity_adjustment' AND asset_id IS NULL AND location_id IS NOT NULL AND requested_quantity IS NOT NULL)
    OR
    (kind = 'asset_update' AND asset_id IS NOT NULL AND requested_quantity IS NULL AND (requested_status IS NOT NULL OR requested_location_id IS NOT NULL))
  ),
  CHECK (
    (status = 'pending' AND reviewer_id IS NULL AND reviewed_at IS NULL)
    OR
    (status IN ('approved', 'rejected') AND reviewer_id IS NOT NULL AND reviewed_at IS NOT NULL)
    OR
    status = 'cancelled'
  )
);

CREATE INDEX idx_adjustment_requests_status_date
  ON adjustment_requests(status, requested_at DESC);
CREATE INDEX idx_adjustment_requests_requester_date
  ON adjustment_requests(requester_id, requested_at DESC);
CREATE INDEX idx_adjustment_requests_product_date
  ON adjustment_requests(product_id, requested_at DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

COMMIT;
