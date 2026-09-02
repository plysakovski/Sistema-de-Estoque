PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

-- O acesso operacional passa a ser explicitamente limitado por unidade.
CREATE TABLE user_locations (
  user_id TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
  location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  assigned_at TEXT NOT NULL,
  PRIMARY KEY (user_id, location_id)
);

CREATE INDEX idx_user_locations_location
  ON user_locations(location_id, user_id);

-- Preserva o comportamento das instalações existentes. Novos vínculos são
-- administrados explicitamente; administradores continuam com escopo global.
INSERT INTO user_locations(user_id, location_id, assigned_at)
SELECT u.id, l.id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM users u
CROSS JOIN locations l
WHERE u.role IN ('operator', 'manager') AND l.active = 1;

-- O nível mínimo deixa de ser uma propriedade global do produto.
CREATE TABLE stock_policies (
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  minimum_quantity INTEGER NOT NULL DEFAULT 0 CHECK (minimum_quantity >= 0),
  target_quantity INTEGER NOT NULL DEFAULT 0 CHECK (target_quantity >= minimum_quantity),
  updated_by TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (product_id, location_id)
);

INSERT INTO stock_policies(
  product_id, location_id, minimum_quantity, target_quantity, updated_at
)
SELECT
  b.product_id,
  b.location_id,
  p.minimum_quantity,
  MAX(p.minimum_quantity, b.quantity),
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM stock_balances b
JOIN products p ON p.id = b.product_id;

-- Reservas impedem que uma transferência aprovada seja prometida duas vezes.
ALTER TABLE stock_balances ADD COLUMN reserved_quantity INTEGER NOT NULL DEFAULT 0
  CHECK (reserved_quantity >= 0 AND reserved_quantity <= quantity);

CREATE TABLE replenishment_requests (
  id TEXT PRIMARY KEY,
  requester_id TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  destination_location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  priority TEXT NOT NULL DEFAULT 'normal'
    CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
  justification TEXT NOT NULL CHECK (length(trim(justification)) >= 10),
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'approved', 'partially_approved', 'rejected', 'in_fulfillment', 'fulfilled', 'cancelled')),
  reviewer_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  review_note TEXT,
  requested_at TEXT NOT NULL,
  reviewed_at TEXT,
  fulfilled_at TEXT,
  version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
  CHECK (
    (status = 'pending' AND reviewer_id IS NULL AND reviewed_at IS NULL)
    OR
    (status <> 'pending' AND status <> 'cancelled' AND reviewer_id IS NOT NULL AND reviewed_at IS NOT NULL)
    OR status = 'cancelled'
  )
);

CREATE TABLE replenishment_request_items (
  id TEXT PRIMARY KEY,
  request_id TEXT NOT NULL REFERENCES replenishment_requests(id) ON UPDATE CASCADE ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  requested_quantity INTEGER NOT NULL CHECK (requested_quantity > 0),
  stock_snapshot INTEGER NOT NULL CHECK (stock_snapshot >= 0),
  approved_quantity INTEGER CHECK (approved_quantity >= 0 AND approved_quantity <= requested_quantity),
  transfer_quantity INTEGER NOT NULL DEFAULT 0 CHECK (transfer_quantity >= 0),
  purchase_quantity INTEGER NOT NULL DEFAULT 0 CHECK (purchase_quantity >= 0),
  source_location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  purchase_reference TEXT,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'approved', 'partially_approved', 'rejected', 'in_fulfillment', 'fulfilled', 'cancelled')),
  version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
  UNIQUE (request_id, product_id),
  CHECK (transfer_quantity + purchase_quantity <= requested_quantity),
  CHECK (transfer_quantity = 0 OR source_location_id IS NOT NULL)
);

CREATE INDEX idx_replenishment_requests_status_date
  ON replenishment_requests(status, requested_at DESC);
CREATE INDEX idx_replenishment_requests_destination_date
  ON replenishment_requests(destination_location_id, requested_at DESC);
CREATE INDEX idx_replenishment_requests_requester_date
  ON replenishment_requests(requester_id, requested_at DESC);
CREATE INDEX idx_replenishment_items_product
  ON replenishment_request_items(product_id, request_id);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

COMMIT;
