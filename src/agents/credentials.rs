//! Owner-issued credentials for external agents. Tokens are shown once and only
//! their SHA-256 is stored; a revoked credential stops working immediately.
use crate::{
    crypto,
    error::{require, Error, Result},
    ontology::validate_text,
    params,
    store::{id, now, Store},
    Actor, AppState,
};
use axum::{extract::State, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const PROTOCOLS: &[(&str, &str)] = &[
    ("http", "HTTP / OpenAPI"),
    ("mcp", "MCP 工具桥"),
    ("other", "其他"),
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
/// The external agent id for a bearer-token digest, if its credential is active.
pub fn authenticate(store: &Store, digest: &[u8; 32]) -> Result<Option<String>> {
    let mut statement = store
        .db
        .prepare("SELECT id FROM agent_credentials WHERE token_hash=$1 AND revoked_at IS NULL")?;
    let found = statement
        .query_map(params![hex(digest)], |r| r.get::<_, String>(0))?
        .next()
        .transpose()?;
    if let Some(agent) = &found {
        store.db.execute(
            "UPDATE agent_credentials SET last_used_at=$1 WHERE id=$2",
            params![now(), agent],
        )?;
    }
    Ok(found)
}
/// (name, protocol) of a credential, revoked or not.
pub fn name_of(store: &Store, agent: &str) -> Result<(String, String)> {
    let mut statement = store
        .db
        .prepare("SELECT name,protocol FROM agent_credentials WHERE id=$1")?;
    statement
        .query_map(params![agent], |r| Ok((r.get(0)?, r.get(1)?)))?
        .next()
        .transpose()?
        .ok_or(Error::NotFound)
}
pub fn connections(store: &Store) -> Result<Vec<Value>> {
    let mut statement = store.db.prepare(
        "SELECT id,name,protocol,created_at,revoked_at,last_used_at FROM agent_credentials ORDER BY created_at DESC",
    )?;
    let rows = statement
        .query_map(&[], |r| {
            let revoked: Option<String> = r.get(4)?;
            Ok(json!({
                "id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "protocol": r.get::<_, String>(2)?,
                "created_at": r.get::<_, String>(3)?, "active": revoked.is_none(), "revoked_at": revoked,
                "last_used_at": r.get::<_, Option<String>>(5)?,
            }))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
pub async fn list(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    Ok(Json(json!({
        "connections": connections(&inner.store)?,
        "protocols": PROTOCOLS.iter().map(|(id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
        "executor_configured": inner.executor_hash.is_some(),
    })))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewConnection {
    pub name: String,
    pub protocol: String,
}
pub async fn create(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<NewConnection>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.name, 80)?;
    require(
        PROTOCOLS.iter().any(|(id, _)| *id == body.protocol),
        "接入方式不合法",
    )?;
    let token = format!("prm_{}", crypto::token());
    let hash = hex(&Sha256::digest(token.as_bytes()));
    let connection = id();
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        store.db.execute(
            "INSERT INTO agent_credentials(id,name,protocol,token_hash,created_at) VALUES($1,$2,$3,$4,$5)",
            params![connection, body.name.trim(), body.protocol, hash, now()],
        )?;
        store.audit("owner", "agent.connection_created", &connection)
    })?;
    Ok(Json(json!({
        "id": connection, "name": body.name.trim(), "protocol": body.protocol, "token": token,
        "notice": "令牌只显示这一次，请立即保存到外部 Agent 的配置中。它只能导入待审信息，不能读取你的数据。",
    })))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeConnection {
    pub id: String,
}
pub async fn revoke(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<RevokeConnection>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        if store.db.execute(
            "UPDATE agent_credentials SET revoked_at=$1 WHERE id=$2 AND revoked_at IS NULL",
            params![now(), body.id],
        )? != 1
        {
            return Err(Error::NotFound);
        }
        store.audit("owner", "agent.connection_revoked", &body.id)
    })?;
    Ok(Json(json!({"ok": true})))
}
