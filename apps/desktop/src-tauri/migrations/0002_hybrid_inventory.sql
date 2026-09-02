PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS products (
  id TEXT PRIMARY KEY,
  sku TEXT NOT NULL UNIQUE COLLATE NOCASE,
  name TEXT NOT NULL,
  category_id TEXT NOT NULL REFERENCES categories(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  tracking_type TEXT NOT NULL CHECK (tracking_type IN ('quantity', 'serialized')),
  minimum_quantity INTEGER NOT NULL DEFAULT 0 CHECK (minimum_quantity >= 0),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  metadata TEXT NOT NULL DEFAULT '{}',
  version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS stock_balances (
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  quantity INTEGER NOT NULL DEFAULT 0 CHECK (quantity >= 0),
  updated_at TEXT NOT NULL,
  PRIMARY KEY (product_id, location_id)
);

CREATE TABLE IF NOT EXISTS assets (
  id TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  asset_tag TEXT NOT NULL UNIQUE COLLATE NOCASE,
  serial_number TEXT UNIQUE COLLATE NOCASE,
  location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  status TEXT NOT NULL DEFAULT 'available' CHECK (status IN ('available', 'in_use', 'maintenance', 'disposed')),
  metadata TEXT NOT NULL DEFAULT '{}',
  version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS stock_movements (
  id TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  asset_id TEXT REFERENCES assets(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  kind TEXT NOT NULL CHECK (kind IN ('entry', 'exit', 'transfer', 'adjustment')),
  quantity INTEGER NOT NULL CHECK (quantity > 0),
  from_location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  to_location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  actor_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
  note TEXT NOT NULL DEFAULT '',
  occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_products_name ON products(name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);
CREATE INDEX IF NOT EXISTS idx_balances_location ON stock_balances(location_id);
CREATE INDEX IF NOT EXISTS idx_assets_product ON assets(product_id);
CREATE INDEX IF NOT EXISTS idx_assets_location_status ON assets(location_id, status);
CREATE INDEX IF NOT EXISTS idx_stock_movements_product_date ON stock_movements(product_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_stock_movements_asset_date ON stock_movements(asset_id, occurred_at DESC);

-- Compatibilidade: instalações da primeira demo são importadas como produtos
-- controlados por quantidade. As tabelas antigas permanecem somente para que a
-- migração seja segura e reversível por backup.
INSERT OR IGNORE INTO products(
  id, sku, name, category_id, tracking_type, minimum_quantity,
  active, metadata, version, created_at, updated_at
)
SELECT
  id, asset_tag, name, category_id, 'quantity', minimum_quantity,
  CASE WHEN status = 'disposed' THEN 0 ELSE 1 END,
  metadata, version, created_at, updated_at
FROM items;

INSERT OR IGNORE INTO stock_balances(product_id, location_id, quantity, updated_at)
SELECT id, location_id, quantity, updated_at
FROM items
WHERE quantity > 0;

INSERT OR IGNORE INTO stock_movements(
  id, product_id, kind, quantity, from_location_id, to_location_id,
  actor_id, note, occurred_at
)
SELECT
  id, item_id, kind, quantity, from_location_id, to_location_id,
  actor_id, note, occurred_at
FROM movements
WHERE EXISTS (SELECT 1 FROM products p WHERE p.id = movements.item_id);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
