PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

ALTER TABLE replenishment_request_items
  ADD COLUMN transfer_received_quantity INTEGER NOT NULL DEFAULT 0
  CHECK (transfer_received_quantity >= 0 AND transfer_received_quantity <= transfer_quantity);

ALTER TABLE replenishment_request_items
  ADD COLUMN transfer_in_transit_quantity INTEGER NOT NULL DEFAULT 0
  CHECK (transfer_in_transit_quantity >= 0 AND transfer_received_quantity + transfer_in_transit_quantity <= transfer_quantity);

ALTER TABLE replenishment_request_items
  ADD COLUMN transfer_rejected_quantity INTEGER NOT NULL DEFAULT 0
  CHECK (transfer_rejected_quantity >= 0);

CREATE TABLE replenishment_transfer_shipments (
  id TEXT PRIMARY KEY,
  request_id TEXT NOT NULL REFERENCES replenishment_requests(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  source_location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  destination_location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  reference TEXT NOT NULL CHECK (length(trim(reference)) > 0),
  dispatch_note TEXT NOT NULL DEFAULT '',
  receipt_reference TEXT,
  receipt_note TEXT,
  status TEXT NOT NULL DEFAULT 'dispatched'
    CHECK (status IN ('dispatched', 'partially_received', 'received', 'divergent')),
  dispatched_by TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  received_by TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  dispatched_at TEXT NOT NULL,
  received_at TEXT,
  version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
  CHECK (source_location_id <> destination_location_id)
);

CREATE TABLE replenishment_transfer_shipment_items (
  id TEXT PRIMARY KEY,
  shipment_id TEXT NOT NULL REFERENCES replenishment_transfer_shipments(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  request_item_id TEXT NOT NULL REFERENCES replenishment_request_items(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  asset_id TEXT REFERENCES assets(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  quantity INTEGER NOT NULL CHECK (quantity > 0),
  received_quantity INTEGER NOT NULL DEFAULT 0 CHECK (received_quantity >= 0),
  rejected_quantity INTEGER NOT NULL DEFAULT 0 CHECK (rejected_quantity >= 0),
  receipt_note TEXT,
  CHECK (received_quantity + rejected_quantity <= quantity),
  CHECK (asset_id IS NULL OR quantity = 1),
  UNIQUE(shipment_id, asset_id)
);

CREATE TABLE replenishment_transfer_asset_reservations (
  request_item_id TEXT NOT NULL REFERENCES replenishment_request_items(id) ON UPDATE CASCADE ON DELETE CASCADE,
  asset_id TEXT NOT NULL UNIQUE REFERENCES assets(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  reserved_at TEXT NOT NULL,
  PRIMARY KEY(request_item_id, asset_id)
);

ALTER TABLE assets ADD COLUMN in_transit_shipment_id TEXT
  REFERENCES replenishment_transfer_shipments(id) ON UPDATE CASCADE ON DELETE RESTRICT;

CREATE INDEX idx_transfer_shipments_request_status
  ON replenishment_transfer_shipments(request_id, status, dispatched_at DESC);
CREATE INDEX idx_transfer_shipments_destination_status
  ON replenishment_transfer_shipments(destination_location_id, status, dispatched_at DESC);
CREATE INDEX idx_transfer_shipment_items_shipment
  ON replenishment_transfer_shipment_items(shipment_id, request_item_id);
CREATE INDEX idx_transfer_asset_reservations_item
  ON replenishment_transfer_asset_reservations(request_item_id);
CREATE INDEX idx_assets_in_transit ON assets(in_transit_shipment_id);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (6, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

COMMIT;
