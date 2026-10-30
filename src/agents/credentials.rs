//! Owner-issued credentials for external agents. Tokens are shown once and only
//! their SHA-256 is stored; a revoked credential stops working immediately.
//! Issuing a token also registers the agent as an `ai_agent` entity in the
//! digital world and opens a relation (in the owner's life scenes) between the
//! self and the agent: connecting an agent *is* establishing that relation.
use crate::{
    agents::registry,
    crypto,
    error::{require, Error, Result},
    ontology::validate_text,
    params, relationships, self_model,
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
/// The relation type a new connection opens unless the owner picks another.
const DEFAULT_RELATIONSHIP_TYPE: &str = "ai_assistant";

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
/// The `ai_agent` entity behind a credential ("" for legacy rows).
pub fn entity_of(store: &Store, agent: &str) -> Result<String> {
    let mut statement = store
        .db
        .prepare("SELECT entity_id FROM agent_credentials WHERE id=$1")?;
    let found = statement
        .query_map(params![agent], |r| r.get::<_, String>(0))?
        .next()
        .transpose()?;
    Ok(found.unwrap_or_default())
}
/// Credential list joined with the agent entity and the relation it opened.
pub fn connections(store: &Store) -> Result<Vec<Value>> {
    let mut statement = store.db.prepare(
        "SELECT id,name,protocol,created_at,revoked_at,last_used_at,entity_id FROM agent_credentials ORDER BY created_at DESC",
    )?;
    let rows = statement
        .query_map(&[], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let entities = store.entities_of(&["ai_agent", "relationship"])?;
    let relations = store.relations()?;
    let find = |kind: &str, entity_id: &str| {
        entities
            .iter()
            .find(|e| e.kind == kind && e.id == entity_id)
            .cloned()
    };
    let mut items = Vec::with_capacity(rows.len());
    for (connection, name, protocol, created_at, revoked_at, last_used_at, entity_id) in rows {
        let agent = (!entity_id.is_empty())
            .then(|| find("ai_agent", &entity_id))
            .flatten();
        let relationship = relations
            .iter()
            .find(|r| r.predicate == "involves" && r.object == entity_id)
            .and_then(|r| find("relationship", &r.subject));
        let platform = agent
            .as_ref()
            .and_then(|a| a.properties["platform"].as_str())
            .unwrap_or("")
            .to_string();
        items.push(json!({
            "id": connection, "name": name, "protocol": protocol,
            "platform": platform,
            "platform_label": registry::platform(&platform).map(|p| p.label).unwrap_or("").to_string(),
            "scenes": agent.as_ref().map(|a| a.scenes.clone()).unwrap_or_default(),
            "agent_id": agent.as_ref().map(|a| a.id.clone()).unwrap_or_default(),
            "relationship_id": relationship.as_ref().map(|r| r.id.clone()).unwrap_or_default(),
            "relationship_status": relationship.as_ref().map(|r| r.status.clone()).unwrap_or_default(),
            "created_at": created_at, "active": revoked_at.is_none(), "revoked_at": revoked_at,
            "last_used_at": last_used_at,
        }));
    }
    Ok(items)
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
        "platforms": registry::platforms_json(),
        "agent_relationship_types": relationships::ai_types().map(|(id, label, scene)| json!({"id": id, "label": label, "scene": scene})).collect::<Vec<_>>(),
        "executor_configured": inner.executor_hash.is_some(),
    })))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewConnection {
    pub name: String,
    pub protocol: String,
    /// Platform preset id (Grok, Claude, Muse, …); empty means "custom".
    #[serde(default)]
    pub platform: String,
    /// One of the AI relation types; empty means "ai_assistant".
    #[serde(default)]
    pub relationship_type: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub notes: String,
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
    let platform_id = if body.platform.is_empty() {
        "custom"
    } else {
        body.platform.as_str()
    };
    registry::agent_properties(platform_id, &body.protocol, &body.notes)?;
    let type_id = if body.relationship_type.is_empty() {
        DEFAULT_RELATIONSHIP_TYPE
    } else {
        body.relationship_type.as_str()
    };
    let (_, type_label, default_scene) =
        *relationships::ai_type(type_id).ok_or(Error::Bad("AI 关系类型不合法".into()))?;
    crate::ontology::validate_scenes(&body.scenes)?;
    let scenes = if body.scenes.is_empty() {
        vec![default_scene.to_string()]
    } else {
        body.scenes.clone()
    };
    let token = format!("prm_{}", crypto::token());
    let hash = hex(&Sha256::digest(token.as_bytes()));
    let connection = id();
    let inner = state.lock()?;
    let (agent, relationship) = inner.store.transaction(|store| {
        let agent = store.insert_scoped(
            "ai_agent",
            body.name.trim(),
            &[],
            &scenes,
            registry::agent_properties(platform_id, &body.protocol, &body.notes)?,
            "active",
        )?;
        let profile = self_model::ensure_self_profile(store)?;
        let relationship = store.insert_scoped(
            "relationship",
            &format!("{} · {}", body.name.trim(), type_label),
            &[],
            &scenes,
            json!({
                "type": type_id, "party_kind": "ai_agent", "platform": platform_id,
                "agent_id": agent.id, "agent_name": agent.title,
                "closeness": 4, "satisfaction": null, "contact_interval_days": 0,
                "last_contact_at": "", "notes": body.notes, "agent_visible": false,
            }),
            "active",
        )?;
        store.relate(&relationship.id, "involves", &agent.id)?;
        store.relate(&relationship.id, "relates_to_self", &profile.id)?;
        store.db.execute(
            "INSERT INTO agent_credentials(id,name,protocol,token_hash,created_at,entity_id) VALUES($1,$2,$3,$4,$5,$6)",
            params![connection, body.name.trim(), body.protocol, hash, now(), agent.id],
        )?;
        store.audit("owner", "agent.connection_created", &connection)?;
        store.audit("owner", "relationship.created", &relationship.id)?;
        Ok((agent, relationship))
    })?;
    Ok(Json(json!({
        "id": connection, "name": body.name.trim(), "protocol": body.protocol,
        "platform": platform_id,
        "platform_label": registry::platform(platform_id).map(|p| p.label).unwrap_or(platform_id),
        "agent_id": agent.id, "relationship_id": relationship.id,
        "token": token, "snippet": registry::snippet(&body.protocol, &token),
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
