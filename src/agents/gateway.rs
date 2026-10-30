//! Import endpoint for external agents. Accepts PRM items (resources, relations,
//! messages, notes, people, tasks) or OpenAI-style chat messages, stores them as
//! an immutable agent_import and dispatches it through the router.
use crate::{
    agents::{credentials, executor, router},
    error::{require, Error, Result},
    ontology::{self, validate_optional, validate_text, Entity},
    store::{now, Store},
    Actor, AppState,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const ITEM_KINDS: &[(&str, &str)] = &[
    ("resource", "资源"),
    ("relation", "关系"),
    ("message", "消息"),
    ("note", "知识"),
    ("person", "人物"),
    ("task", "任务"),
];

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ImportItem {
    #[serde(rename = "ref", default)]
    pub reference: String,
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub predicate: String,
    #[serde(default)]
    pub object: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,
    #[serde(default)]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub conversation: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub items: Vec<ImportItem>,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}
fn reference_ok(value: &str, max: usize) -> bool {
    value.chars().count() <= max
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
/// Text of a chat message: a string, or the text parts of a content array.
fn content_text(content: &Value) -> Option<String> {
    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let texts: Vec<&str> = parts
                .iter()
                .filter(|part| part["type"] == "text")
                .filter_map(|part| part["text"].as_str())
                .collect();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        _ => None,
    }
}
/// Validate an import and convert chat messages into message items.
pub fn normalize(request: ImportRequest) -> Result<(Vec<ImportItem>, &'static str)> {
    validate_optional(&request.conversation, 200, "会话引用过长")?;
    ontology::validate_scenes(&request.scenes)?;
    let format = match (request.items.is_empty(), request.messages.is_empty()) {
        (false, true) => "prm-import-v1",
        (true, false) => "chat-messages-v1",
        _ => return Err(Error::Bad("请提供 items 或 messages 其中之一".into())),
    };
    require(
        request.format.is_empty() || request.format == format,
        "format 与提交内容不一致",
    )?;
    let items = if format == "prm-import-v1" {
        request.items
    } else {
        require(request.messages.len() <= 100, "单次最多导入 100 条消息")?;
        request
            .messages
            .iter()
            .filter(|m| ["user", "assistant"].contains(&m.role.as_str()))
            .filter_map(|m| content_text(&m.content).map(|text| (m, text)))
            .filter(|(_, text)| !text.trim().is_empty())
            .map(|(m, text)| {
                let mut attributes = BTreeMap::from([("role".to_string(), m.role.clone())]);
                if !m.name.is_empty() {
                    attributes.insert("name".into(), m.name.chars().take(80).collect());
                }
                ImportItem {
                    reference: String::new(),
                    kind: "message".into(),
                    title: text.trim().chars().take(100).collect(),
                    body: text,
                    scenes: Vec::new(),
                    attributes,
                    subject: String::new(),
                    predicate: String::new(),
                    object: String::new(),
                }
            })
            .collect()
    };
    require(
        !items.is_empty() && items.len() <= 100,
        "导入项目数量应为 1–100",
    )?;
    for (index, item) in items.iter().enumerate() {
        require(
            ITEM_KINDS.iter().any(|(kind, _)| *kind == item.kind),
            "导入类型不合法",
        )?;
        if item.kind == "relation" {
            validate_optional(&item.title, 160, "导入标题过长")?;
            require(
                !item.subject.is_empty()
                    && !item.object.is_empty()
                    && reference_ok(&item.predicate, 60)
                    && item.subject.chars().count() <= 80
                    && item.object.chars().count() <= 80,
                "关系项需要 subject、predicate、object",
            )?;
        } else {
            validate_text(&item.title, 160)?;
        }
        validate_optional(&item.body, 8000, "导入正文过长")?;
        ontology::validate_scenes(&item.scenes)?;
        require(
            reference_ok(&item.reference, 40)
                && (item.reference.is_empty()
                    || items[..index].iter().all(|o| o.reference != item.reference)),
            "导入项编号不合法或重复",
        )?;
        require(
            item.attributes.len() <= 20
                && item.attributes.iter().all(|(key, value)| {
                    !key.is_empty()
                        && key.len() <= 40
                        && key
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                        && value.chars().count() <= 400
                }),
            "导入属性不合法",
        )?;
    }
    Ok((items, format))
}
pub async fn import(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<ImportRequest>,
) -> Result<Json<Value>> {
    let agent = actor.external()?.to_string();
    let scenes = body.scenes.clone();
    let conversation = body.conversation.clone();
    let (items, format) = normalize(body)?;
    let inner = state.lock()?;
    let (name, protocol) = credentials::name_of(&inner.store, &agent)?;
    let agent_entity = credentials::entity_of(&inner.store, &agent)?;
    let outcome = inner.store.transaction(|store| {
        let mut properties = json!({
            "contract": "prm-import-v2", "agent_id": agent, "agent_name": name, "protocol": protocol,
            "format": format, "conversation": conversation, "items": items, "item_count": items.len(),
            "received_at": now(), "review_required": true,
        });
        if !agent_entity.is_empty() {
            properties["agent_entity_id"] = json!(&agent_entity);
        }
        let import = store.insert_scoped(
            "agent_import",
            &format!("来自 {name} 的导入"),
            &[],
            &scenes,
            properties,
            "queued",
        )?;
        if !agent_entity.is_empty() {
            store.relate(&import.id, "from_agent", &agent_entity)?;
        }
        store.audit("external", "agent.import_received", &import.id)?;
        let route = router::route(store, &agent, &scenes, &items)?;
        executor::dispatch(store, &import, &items, &route)
    })?;
    Ok(Json(outcome))
}
/// Status only: an external agent never sees what the router or the owner made of it.
fn summary(import: &Entity, proposals: &[Entity]) -> Value {
    let count = |status: &str| {
        proposals
            .iter()
            .filter(|p| p.properties["source_id"] == import.id && p.status == status)
            .count()
    };
    json!({
        "id": import.id, "status": import.status,
        "received_at": import.properties["received_at"], "item_count": import.properties["item_count"],
        "proposals": {"pending": count("pending"), "approved": count("approved"), "rejected": count("rejected")},
    })
}
fn own_imports(store: &Store, agent: &str) -> Result<Vec<Entity>> {
    Ok(store
        .entities_of(&["agent_import"])?
        .into_iter()
        .filter(|i| i.properties["agent_id"] == agent)
        .collect())
}
pub async fn imports(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    let agent = actor.external()?;
    let inner = state.lock()?;
    let proposals = inner.store.entities_of(&["proposal"])?;
    let list: Vec<Value> = own_imports(&inner.store, agent)?
        .iter()
        .take(50)
        .map(|import| summary(import, &proposals))
        .collect();
    Ok(Json(json!({"imports": list})))
}
pub async fn import_status(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let agent = actor.external()?;
    let inner = state.lock()?;
    let import = own_imports(&inner.store, agent)?
        .into_iter()
        .find(|i| i.id == id)
        .ok_or(Error::NotFound)?;
    let proposals = inner.store.entities_of(&["proposal"])?;
    Ok(Json(summary(&import, &proposals)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(value: Value) -> ImportRequest {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn chat_messages_become_message_items() {
        let (items, format) = normalize(request(json!({"messages": [
            {"role": "system", "content": "ignored"},
            {"role": "user", "content": "明天整理报销单"},
            {"role": "assistant", "content": [{"type": "text", "text": "好的"}]},
        ]})))
        .unwrap();
        assert_eq!(format, "chat-messages-v1");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].attributes["role"], "user");
        assert!(items.iter().all(|i| i.kind == "message"));
    }

    #[test]
    fn malformed_imports_are_rejected() {
        assert!(normalize(request(json!({}))).is_err());
        assert!(normalize(request(
            json!({"items": [{"kind": "payment", "title": "x"}]})
        ))
        .is_err());
        assert!(normalize(request(
            json!({"items": [{"kind": "relation", "title": "", "subject": "a"}]})
        ))
        .is_err());
        assert!(normalize(request(json!({"items": [
            {"ref": "a", "kind": "note", "title": "x"}, {"ref": "a", "kind": "note", "title": "y"}
        ]})))
        .is_err());
        assert!(normalize(request(
            json!({"items": [{"kind": "note", "title": "x", "attributes": {"Bad Key": "v"}}]})
        ))
        .is_err());
        assert!(
            serde_json::from_value::<ImportRequest>(json!({"items": [], "command": "pay"}))
                .is_err()
        );
    }
}
