-- Runs on every startup: every statement must be idempotent.
-- ontology_classes rows and relations_predicate_check are synced from src/ontology.rs afterwards.
CREATE TABLE IF NOT EXISTS account (id BIGINT PRIMARY KEY CHECK (id=1), password_hash TEXT NOT NULL, vault_salt BYTEA NOT NULL);
CREATE TABLE IF NOT EXISTS vault (id BIGINT PRIMARY KEY CHECK(id=1), payload BYTEA NOT NULL);
CREATE TABLE IF NOT EXISTS ontology_classes (id TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS entities (
 id TEXT PRIMARY KEY, kind TEXT NOT NULL REFERENCES ontology_classes(id), title TEXT NOT NULL,
 worlds TEXT NOT NULL CHECK (jsonb_typeof(worlds::jsonb)='array'), value_axes TEXT NOT NULL CHECK (jsonb_typeof(value_axes::jsonb)='array'), properties TEXT NOT NULL CHECK (jsonb_typeof(properties::jsonb)='object'),
 status TEXT NOT NULL, version BIGINT NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
ALTER TABLE entities ADD COLUMN IF NOT EXISTS scenes TEXT NOT NULL DEFAULT '[]';
ALTER TABLE entities DROP CONSTRAINT IF EXISTS entities_scenes_check;
ALTER TABLE entities ADD CONSTRAINT entities_scenes_check CHECK (jsonb_typeof(scenes::jsonb)='array');
CREATE INDEX IF NOT EXISTS idx_entity_kind ON entities(kind,created_at);
CREATE TABLE IF NOT EXISTS relations (id TEXT PRIMARY KEY, subject TEXT NOT NULL REFERENCES entities(id), predicate TEXT NOT NULL, object TEXT NOT NULL REFERENCES entities(id), UNIQUE(subject,predicate,object), CHECK(subject<>object));
CREATE TABLE IF NOT EXISTS audit (sequence BIGSERIAL PRIMARY KEY, at TEXT NOT NULL, actor TEXT NOT NULL, action TEXT NOT NULL, target TEXT NOT NULL);
CREATE OR REPLACE FUNCTION reject_audit_change() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'audit is append only'; END $$;
DROP TRIGGER IF EXISTS audit_immutable ON audit;
CREATE TRIGGER audit_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON audit FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_change();
CREATE TABLE IF NOT EXISTS read_grants (id TEXT PRIMARY KEY, public_key TEXT NOT NULL, label TEXT NOT NULL, item_ids TEXT NOT NULL CHECK(jsonb_typeof(item_ids::jsonb)='array'), expires_at BIGINT NOT NULL, revoked BOOLEAN NOT NULL DEFAULT FALSE, created_at TEXT NOT NULL);

CREATE TABLE IF NOT EXISTS request_nonces (id TEXT PRIMARY KEY, expires_at BIGINT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_nonce_expiry ON request_nonces(expires_at);
-- Owner-issued credentials for external agents. Only the SHA-256 of each token is stored.
CREATE TABLE IF NOT EXISTS agent_credentials (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, protocol TEXT NOT NULL, token_hash TEXT NOT NULL UNIQUE,
 created_at TEXT NOT NULL, revoked_at TEXT, last_used_at TEXT
);
-- Never used: agent runs are agent_run entities.
DROP TABLE IF EXISTS agent_runs;
