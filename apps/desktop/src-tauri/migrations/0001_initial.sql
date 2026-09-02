PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE COLLATE NOCASE,
  display_name TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('admin', 'manager', 'operator', 'viewer')),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS locations (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  code TEXT NOT NULL UNIQUE COLLATE NOCASE,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS categories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  code TEXT NOT NULL UNIQUE COLLATE NOCASE,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
  id TEXT PRIMARY KEY,
  asset_tag TEXT NOT NULL UNIQUE COLLATE NOCASE,
  name TEXT NOT NULL,
  category_id TEXT NOT NULL REFERENCES categories(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  quantity INTEGER NOT NULL DEFAULT 0 CHECK (quantity >= 0),
  minimum_quantity INTEGER NOT NULL DEFAULT 0 CHECK (minimum_quantity >= 0),
  status TEXT NOT NULL DEFAULT 'available' CHECK (status IN ('available', 'in_use', 'maintenance', 'disposed')),
  metadata TEXT NOT NULL DEFAULT '{}',
  version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS movements (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES items(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  kind TEXT NOT NULL CHECK (kind IN ('entry', 'exit', 'transfer', 'adjustment')),
  quantity INTEGER NOT NULL CHECK (quantity > 0),
  from_location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  to_location_id TEXT REFERENCES locations(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  actor_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
  note TEXT NOT NULL DEFAULT '',
  occurred_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
  id TEXT PRIMARY KEY,
  actor_id TEXT REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
  action TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  before_data TEXT,
  after_data TEXT,
  occurred_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_items_name ON items(name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_items_category ON items(category_id);
CREATE INDEX IF NOT EXISTS idx_items_location ON items(location_id);
CREATE INDEX IF NOT EXISTS idx_movements_item_date ON movements(item_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_entity_date ON audit_logs(entity_type, entity_id, occurred_at DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
