PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS account (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL,
    vault_salt BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS ontology_classes (id TEXT PRIMARY KEY);
INSERT OR IGNORE INTO ontology_classes(id) VALUES
 ('directive'), ('channel'), ('message'), ('note'), ('proposal'),
 ('task'), ('resource'), ('stock_movement'), ('cash_entry'), ('self_profile'),
 ('body_metric'), ('psychological_metric'), ('self_relation'), ('supplier'),
 ('purchase_order'), ('purchase_order_line'), ('agent_run'), ('agent_import');
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL REFERENCES ontology_classes(id),
    title TEXT NOT NULL,
    worlds TEXT NOT NULL CHECK (json_valid(worlds)),
    value_axes TEXT NOT NULL CHECK (json_valid(value_axes)),
    properties TEXT NOT NULL CHECK (json_valid(properties)),
    status TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_entity_kind ON entities(kind, created_at);
CREATE TABLE IF NOT EXISTS relations (
    id TEXT PRIMARY KEY,
    subject TEXT NOT NULL REFERENCES entities(id),
    predicate TEXT NOT NULL CHECK (predicate IN ('implements','uses','references','derived_from','belongs_to','records','describes_self','relates_to_self','measures','supports','influences','supplies','contains')),
    object TEXT NOT NULL REFERENCES entities(id),
    UNIQUE(subject, predicate, object),
    CHECK(subject <> object)
);
CREATE TABLE IF NOT EXISTS audit (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    at TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS audit_no_update BEFORE UPDATE ON audit BEGIN SELECT RAISE(ABORT, 'audit is append only'); END;
CREATE TRIGGER IF NOT EXISTS audit_no_delete BEFORE DELETE ON audit BEGIN SELECT RAISE(ABORT, 'audit is append only'); END;
PRAGMA user_version = 1;
