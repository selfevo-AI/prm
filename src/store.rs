use crate::{database::Connection, params};
use crate::{
    error::{Error, Result},
    ontology::{self, Entity, Relation},
};
use serde_json::{json, Value};
use std::path::Path;

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
const ENTITY_COLUMNS: &str =
    "id,kind,title,worlds,value_axes,scenes,properties,status,version,created_at,updated_at";
type RawEntity = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
);
fn raw(r: &crate::database::Row) -> std::result::Result<RawEntity, postgres::Error> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get(9)?,
        r.get(10)?,
    ))
}
fn parse(row: RawEntity) -> Result<Entity> {
    let (
        id,
        kind,
        title,
        worlds,
        values,
        scenes,
        properties,
        status,
        version,
        created_at,
        updated_at,
    ) = row;
    Ok(Entity {
        id,
        kind,
        title,
        worlds: serde_json::from_str(&worlds)?,
        values: serde_json::from_str(&values)?,
        scenes: serde_json::from_str(&scenes)?,
        properties: serde_json::from_str(&properties)?,
        status,
        version,
        created_at,
        updated_at,
    })
}
pub struct Store {
    pub db: Connection,
    _process_lock: std::fs::File,
}
impl Store {
    pub fn open(path: &Path) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(path)?;
        let process_lock = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.join("prm.lock"))?;
        process_lock.try_lock().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "This PRM data directory is already open in another process.",
            )
        })?;
        let db = Connection::open()?;
        use sha2::{Digest, Sha256};
        let identity = std::fs::canonicalize(path)?;
        let digest = Sha256::digest(identity.to_string_lossy().as_bytes());
        let schema = format!(
            "prm_{}",
            digest[..12]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let schema = std::env::var("PRM_DATABASE_SCHEMA").unwrap_or(schema);
        if !schema.starts_with("prm_")
            || schema.len() > 63
            || !schema
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err("PRM_DATABASE_SCHEMA must start with prm_ and contain only lowercase letters, digits and underscores (max 63 bytes)".into());
        }
        db.execute_batch(&format!("CREATE SCHEMA IF NOT EXISTS {schema}; SET search_path TO {schema}; SET statement_timeout='10s'; SET lock_timeout='5s';"))?;
        let locked: bool = db.query_row(
            "SELECT pg_try_advisory_lock(hashtext($1))",
            params![schema],
            |r| r.get(0),
        )?;
        if !locked {
            return Err("This PostgreSQL workspace is already open".into());
        }
        db.execute_batch(include_str!("../migrations/002_postgresql.sql"))?;
        db.execute_batch(&ontology::sync_sql())?;
        crate::legacy::import(&db, path)?;
        Ok(Self {
            db,
            _process_lock: process_lock,
        })
    }
    pub fn initialized(&self) -> Result<bool> {
        Ok(self
            .db
            .query_row("SELECT EXISTS(SELECT 1 FROM account)", &[], |r| r.get(0))?)
    }
    pub fn credentials(&self) -> Result<(String, Vec<u8>)> {
        if !self.initialized()? {
            return Err(Error::Unauthorized);
        }
        Ok(self.db.query_row(
            "SELECT password_hash,vault_salt FROM account WHERE id=1",
            &[],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }
    pub fn entities(&self) -> Result<Vec<Entity>> {
        let mut statement = self.db.prepare(&format!(
            "SELECT {ENTITY_COLUMNS} FROM entities ORDER BY created_at DESC, id"
        ))?;
        let rows = statement.query_map(&[], raw)?;
        rows.map(|row| parse(row?)).collect()
    }
    /// Entities of the given kinds, newest first.
    pub fn entities_of(&self, kinds: &[&str]) -> Result<Vec<Entity>> {
        let kinds: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
        let mut statement = self.db.prepare(&format!(
            "SELECT {ENTITY_COLUMNS} FROM entities WHERE kind = ANY($1) ORDER BY created_at DESC, id"
        ))?;
        let rows = statement.query_map(params![kinds], raw)?;
        rows.map(|row| parse(row?)).collect()
    }
    pub fn entity(&self, entity_id: &str) -> Result<Entity> {
        let mut statement = self.db.prepare(&format!(
            "SELECT {ENTITY_COLUMNS} FROM entities WHERE id=$1"
        ))?;
        let mut rows = statement.query_map(params![entity_id], raw)?;
        match rows.next() {
            Some(row) => parse(row?),
            None => Err(Error::NotFound),
        }
    }
    /// Insert with the class's default scenes.
    pub fn insert(
        &self,
        kind: &str,
        title: &str,
        values: &[String],
        props: Value,
        status: &str,
    ) -> Result<Entity> {
        self.insert_scoped(
            kind,
            title,
            values,
            &ontology::scenes_for(kind, &[])?,
            props,
            status,
        )
    }
    pub fn insert_scoped(
        &self,
        kind: &str,
        title: &str,
        values: &[String],
        scenes: &[String],
        props: Value,
        status: &str,
    ) -> Result<Entity> {
        let entity = Entity {
            id: id(),
            kind: kind.into(),
            title: title.trim().into(),
            worlds: ontology::worlds(kind),
            values: values.to_vec(),
            scenes: scenes.to_vec(),
            properties: props,
            status: status.into(),
            version: 1,
            created_at: now(),
            updated_at: now(),
        };
        self.db.execute(
            "INSERT INTO entities(id,kind,title,worlds,value_axes,scenes,properties,status,version,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,1,$9,$10)",
            params![
                entity.id,
                entity.kind,
                entity.title,
                serde_json::to_string(&entity.worlds)?,
                serde_json::to_string(values)?,
                serde_json::to_string(scenes)?,
                entity.properties.to_string(),
                status,
                entity.created_at,
                entity.updated_at
            ],
        )?;
        Ok(entity)
    }
    /// Optimistic update: fails with 409 unless `entity.version` is still current.
    pub fn update(&self, entity: &Entity) -> Result<Entity> {
        let updated_at = now();
        let changed = self.db.execute(
            "UPDATE entities SET title=$1,properties=$2,status=$3,scenes=$4,version=version+1,updated_at=$5 WHERE id=$6 AND version=$7",
            params![
                entity.title,
                entity.properties.to_string(),
                entity.status,
                serde_json::to_string(&entity.scenes)?,
                updated_at,
                entity.id,
                entity.version
            ],
        )?;
        if changed != 1 {
            return Err(Error::Conflict("记录已更新，请刷新后重试".into()));
        }
        let mut next = entity.clone();
        next.version += 1;
        next.updated_at = updated_at;
        Ok(next)
    }
    pub fn relations(&self) -> Result<Vec<Relation>> {
        let mut statement = self
            .db
            .prepare("SELECT id,subject,predicate,object FROM relations")?;
        let items = statement
            .query_map(&[], |r| {
                Ok(Relation {
                    id: r.get(0)?,
                    subject: r.get(1)?,
                    predicate: r.get(2)?,
                    object: r.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items)
    }
    pub fn relate(&self, subject: &str, predicate: &str, object: &str) -> Result<Relation> {
        let s = self.entity(subject)?;
        let o = self.entity(object)?;
        if predicate == "implements" && o.status != "active" {
            return Err(Error::Bad("执行方向已撤回".into()));
        }
        if subject == object || !ontology::relation_valid(&s.kind, predicate, &o.kind) {
            return Err(Error::Bad("关系不符合本体定义的类型约束".into()));
        }
        let relation = Relation {
            id: id(),
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
        };
        self.db.execute(
            "INSERT INTO relations(id,subject,predicate,object) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            params![relation.id, subject, predicate, object],
        )?;
        Ok(relation)
    }
    pub fn audit(&self, actor: &str, action: &str, target: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO audit(at,actor,action,target) VALUES ($1,$2,$3,$4)",
            params![now(), actor, action, target],
        )?;
        Ok(())
    }
    pub fn audits(&self) -> Result<Vec<Value>> {
        let mut stmt = self.db.prepare(
            "SELECT sequence,at,actor,action,target FROM audit ORDER BY sequence DESC LIMIT 200",
        )?;
        let items=stmt.query_map(&[], |r| Ok(json!({"sequence":r.get::<_,i64>(0)?,"at":r.get::<_,String>(1)?,"actor":r.get::<_,String>(2)?,"action":r.get::<_,String>(3)?,"target":r.get::<_,String>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(items)
    }
    pub fn transaction<T>(&self, work: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        self.db.execute_batch("BEGIN")?;
        match work(self) {
            Ok(value) => {
                if let Err(error) = self.db.execute_batch("COMMIT") {
                    let _ = self.db.execute_batch("ROLLBACK");
                    return Err(error.into());
                }
                Ok(value)
            }
            Err(error) => {
                let _ = self.db.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }
}
