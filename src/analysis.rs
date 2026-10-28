//! Decisions: offline analyzer rules, the unified proposal queue and owner
//! approval. Every AI or agent output becomes a pending proposal; only the
//! owner turns a proposal into a task, an entity or a relation.
use crate::{
    error::{require, Error, Result},
    ontology::{self, validate_optional, validate_text, validate_values, Entity},
    operations, relationships,
    store::Store,
    Actor, AppState,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const OFFLINE: &str = "offline-rules";

/// Whether `rule` already proposed something for this source version.
fn already_proposed(entities: &[Entity], source: &Entity, rule: &str) -> bool {
    entities.iter().any(|e| {
        e.kind == "proposal"
            && e.properties["source_id"] == source.id
            && e.properties["source_version"].as_i64() == Some(source.version)
            && e.properties["rule"].as_str().is_none_or(|r| r == rule)
    })
}
/// Whether anything proposed for this source version.
pub fn proposed_any(entities: &[Entity], source: &Entity) -> bool {
    entities.iter().any(|e| {
        e.kind == "proposal"
            && e.properties["source_id"] == source.id
            && e.properties["source_version"].as_i64() == Some(source.version)
    })
}
/// A create_task proposal derived from a message, resource or relationship.
pub fn proposal(
    store: &Store,
    source: &Entity,
    title: &str,
    rationale: &str,
    provider: &str,
    rule: &str,
) -> Result<Entity> {
    let entity = store.insert_scoped(
        "proposal",
        title,
        &source.values,
        &source.scenes,
        json!({
            "action":"create_task","source_id":source.id,"source_version":source.version,"source_kind":source.kind,
            "rationale":rationale,"provider":provider,"rule":rule,"risk":"review-required"
        }),
        "pending",
    )?;
    store.relate(&entity.id, "derived_from", &source.id)?;
    store.audit(
        if provider == OFFLINE {
            "agent"
        } else {
            "executor"
        },
        "proposal.created",
        &entity.id,
    )?;
    Ok(entity)
}
fn number(entity: &Entity, key: &str) -> i64 {
    entity.properties[key].as_i64().unwrap_or(0)
}
fn text<'a>(entity: &'a Entity, key: &str) -> &'a str {
    entity.properties[key].as_str().unwrap_or("")
}
fn short(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
fn low_stock_reason(source: &Entity) -> String {
    let unit = text(source, "unit");
    let mut reason = format!(
        "当前库存 {} {unit}，低于你设置的最低库存 {}。",
        number(source, "quantity"),
        number(source, "minimum")
    );
    if number(source, "reorder_qty") > 0 {
        reason += &format!("建议补货量 {} {unit}。", number(source, "reorder_qty"));
    }
    if !text(source, "supplier").is_empty() {
        reason += &format!("首选供应商：{}", text(source, "supplier"));
        if number(source, "lead_time_days") > 0 {
            reason += &format!("，交货周期约 {} 天", number(source, "lead_time_days"));
        }
        reason += "。";
    }
    reason + "建议核实需求、规格和预算后再建立采购订单。"
}
fn analyze_store(store: &Store) -> Result<usize> {
    let entities = store.entities()?;
    let today = chrono::Local::now().date_naive();
    let mut count = 0;
    for source in &entities {
        let mut propose = |rule: &str, title: String, rationale: String| -> Result<()> {
            if !already_proposed(&entities, source, rule) {
                proposal(store, source, &title, &rationale, OFFLINE, rule)?;
                count += 1;
            }
            Ok(())
        };
        match source.kind.as_str() {
            "resource" => {
                if number(source, "quantity") < number(source, "minimum") {
                    propose(
                        "low_stock",
                        format!("检查补充：{}", short(&source.title, 90)),
                        low_stock_reason(source),
                    )?;
                }
                let warranty = chrono::NaiveDate::parse_from_str(text(source, "warranty_until"), "%Y-%m-%d");
                if let (Ok(until), "asset") = (warranty, text(source, "item_type")) {
                    let left = (until - today).num_days();
                    if (0..=30).contains(&left) {
                        propose(
                            "warranty",
                            format!("保修检查：{}", short(&source.title, 90)),
                            format!("保修将在 {until} 到期（剩余 {left} 天）。如需检修、延保或记录设备状况，请在到期前安排。"),
                        )?;
                    }
                }
            }
            "message" if source.properties["role"] == "owner" => propose(
                "owner_message",
                format!("跟进：{}", short(&source.title, 80)),
                "根据这条消息生成待办草案。离线规则仅做收集与转交，内容准确性、优先级和是否执行需要你判断。".into(),
            )?,
            "relationship" => {
                if let Some(days) = relationships::contact_due(source, today) {
                    propose(
                        "contact_due",
                        format!("联系：{}", short(text(source, "person_name"), 80)),
                        format!(
                            "距离上次联系已 {days} 天，超过你设定的 {} 天联系节奏。是否联系、以何种方式联系由你决定。",
                            number(source, "contact_interval_days")
                        ),
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(count)
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub channel_id: String,
    pub body: String,
}
pub async fn message(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Message>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.body, 8000)?;
    let inner = state.lock()?;
    require(
        inner.store.entity(&body.channel_id)?.kind == "channel",
        "会话不存在",
    )?;
    let count=inner.store.transaction(|store| {
        let title=body.body.chars().take(100).collect::<String>();
        let message=store.insert("message",&title,&[],json!({"role":"owner","body":body.body,"channel_id":body.channel_id}),"sent")?;
        store.relate(&message.id,"belongs_to",&body.channel_id)?;
        store.audit("owner","message.created",&message.id)?;
        let count=analyze_store(store)?;
        let reply=store.insert("message","已整理为待办建议",&[],json!({"role":"assistant","provider":OFFLINE,"body":"已保留原始消息，并整理为待办建议。你可以在决策中心选择执行方向、修改标题，再决定是否创建任务。","channel_id":body.channel_id}),"sent")?;
        store.relate(&reply.id,"belongs_to",&body.channel_id)?;
        Ok(count)
    })?;
    Ok(Json(json!({"proposals_created":count})))
}
pub async fn analyze(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    let count = inner.store.transaction(analyze_store)?;
    Ok(Json(json!({"proposals_created":count,"provider":OFFLINE})))
}

/// One structured suggestion from the offline rules or a local model.
/// `subject` / `object` of a relation are entity ids or `#ref` of a candidate
/// in the same batch.
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    #[serde(rename = "ref", default)]
    pub reference: String,
    pub action: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "ontology::empty_object")]
    pub properties: Value,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub predicate: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub principles: Vec<String>,
}
pub struct Staged {
    pub created: usize,
    pub rejected: Vec<Value>,
}
/// Ids the local model may cite as principles: published directives and
/// identity statements the owner made agent-visible.
fn principle_titles(store: &Store) -> Result<HashMap<String, String>> {
    Ok(store
        .entities_of(&["directive", "identity"])?
        .into_iter()
        .filter(|e| {
            (e.kind == "directive" && e.status == "active")
                || (e.kind == "identity" && e.properties["agent_visible"] == true)
        })
        .map(|e| (e.id, e.title))
        .collect())
}
type Prepared = (String, Vec<String>, Vec<String>, Value);
fn prepare(
    store: &Store,
    c: &Candidate,
    staged: &HashMap<String, (String, String)>,
    principles: &HashMap<String, String>,
) -> Result<Prepared> {
    validate_text(&c.rationale, 4000)?;
    validate_values(&c.values)?;
    validate_optional(&c.body, 4000, "候选内容过长")?;
    let cited: Vec<Value> = c
        .principles
        .iter()
        .take(10)
        .filter_map(|id| {
            principles
                .get(id)
                .map(|title| json!({"id": id, "title": title}))
        })
        .collect();
    match c.action.as_str() {
        "create_task" => {
            validate_text(&c.title, 120)?;
            Ok((
                c.title.trim().into(),
                c.values.clone(),
                ontology::scenes_for("task", &c.scenes)?,
                json!({"action": "create_task", "body": c.body, "rationale": c.rationale, "principles": cited}),
            ))
        }
        "create_entity" => {
            require(
                operations::AGENT_CREATABLE.contains(&c.kind.as_str()),
                "候选实体类型不允许",
            )?;
            validate_text(&c.title, 120)?;
            let scenes = ontology::scenes_for(&c.kind, &c.scenes)?;
            // Dry run: approval validates the same input again against the then-current data.
            operations::normalize(store, &c.kind, c.properties.clone())?;
            Ok((
                c.title.trim().into(),
                c.values.clone(),
                scenes,
                json!({"action": "create_entity", "entity": {"kind": c.kind, "properties": c.properties}, "body": c.body, "rationale": c.rationale, "principles": cited}),
            ))
        }
        "create_relation" => {
            let def = ontology::predicate_def(&c.predicate)
                .filter(|d| d.agent_proposable)
                .ok_or(Error::Bad("候选关系类型不允许".into()))?;
            let endpoint = |reference: &str| -> Result<(String, String)> {
                match reference.strip_prefix('#') {
                    Some(local) => staged
                        .get(local)
                        .cloned()
                        .ok_or(Error::Bad("引用的候选不存在或未通过校验".into())),
                    None => store.entity(reference).map(|e| (e.kind, e.title)),
                }
            };
            let (subject_kind, subject_title) = endpoint(&c.subject)?;
            let (object_kind, object_title) = endpoint(&c.object)?;
            require(
                c.subject != c.object
                    && ontology::relation_valid(&subject_kind, def.id, &object_kind),
                "候选关系不符合本体约束",
            )?;
            ontology::validate_scenes(&c.scenes)?;
            Ok((
                short(
                    &format!("{subject_title} {} {object_title}", def.label),
                    120,
                ),
                c.values.clone(),
                c.scenes.clone(),
                json!({"action": "create_relation", "relation": {"subject": c.subject, "predicate": def.id, "object": c.object, "subject_title": subject_title, "object_title": object_title}, "rationale": c.rationale, "principles": cited}),
            ))
        }
        _ => Err(Error::Bad("候选动作不合法".into())),
    }
}
/// Validate candidates and turn the valid ones into pending proposals derived
/// from the import. Invalid candidates are reported, never partially applied.
pub fn stage_candidates(
    store: &Store,
    import: &Entity,
    run: &Entity,
    provider: &str,
    candidates: &[Candidate],
) -> Result<Staged> {
    require(candidates.len() <= 50, "单次最多提交 50 条候选")?;
    let principles = principle_titles(store)?;
    let mut staged: HashMap<String, (String, String)> = HashMap::new();
    let mut created = 0;
    let mut rejected = Vec::new();
    // Entities first, so relations can refer to candidates that passed validation.
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by_key(|&i| candidates[i].action == "create_relation");
    for index in order {
        let c = &candidates[index];
        let reference_ok = c.reference.chars().count() <= 40
            && c.reference
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
            && (c.reference.is_empty()
                || candidates
                    .iter()
                    .filter(|o| o.reference == c.reference)
                    .count()
                    == 1);
        let prepared = if reference_ok {
            prepare(store, c, &staged, &principles)
        } else {
            Err(Error::Bad("候选编号不合法或重复".into()))
        };
        match prepared {
            Ok((title, values, scenes, mut props)) => {
                props["source_id"] = json!(import.id);
                props["source_version"] = json!(import.version);
                props["source_kind"] = json!("agent_import");
                props["agent_name"] = import.properties["agent_name"].clone();
                props["provider"] = json!(provider);
                props["run_id"] = json!(run.id);
                props["candidate_ref"] = json!(c.reference);
                props["rule"] = json!("agent-import");
                props["risk"] = json!("review-required");
                let p = store.insert_scoped("proposal", &title, &values, &scenes, props, "pending")?;
                store.relate(&p.id, "derived_from", &import.id)?;
                store.audit(
                    if provider == OFFLINE { "agent" } else { "executor" },
                    "proposal.created",
                    &p.id,
                )?;
                if c.action == "create_entity" && !c.reference.is_empty() {
                    staged.insert(c.reference.clone(), (c.kind.clone(), title));
                }
                created += 1;
            }
            Err(Error::Bad(reason)) | Err(Error::Conflict(reason)) => {
                rejected.push(json!({"ref": c.reference, "index": index, "reason": reason}))
            }
            Err(Error::NotFound) | Err(Error::Forbidden) => rejected.push(
                json!({"ref": c.reference, "index": index, "reason": "引用的实体不存在或类型不允许"}),
            ),
            Err(error) => return Err(error),
        }
    }
    Ok(Staged { created, rejected })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub decision: String,
    pub expected_version: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub directive_id: String,
}
/// Resolve a relation endpoint: an entity id, or `#ref` of an already approved
/// candidate from the same run.
fn resolve(store: &Store, proposal: &Entity, reference: &str) -> Result<String> {
    let Some(local) = reference.strip_prefix('#') else {
        store.entity(reference)?;
        return Ok(reference.to_string());
    };
    let run = proposal.properties["run_id"].as_str().unwrap_or("");
    let sibling = store
        .entities_of(&["proposal"])?
        .into_iter()
        .find(|e| e.properties["run_id"] == run && e.properties["candidate_ref"] == local)
        .ok_or(Error::Bad("关联的候选不存在".into()))?;
    sibling.properties["created_entity_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .ok_or_else(|| Error::Conflict(format!("请先采纳关联的候选：{}", sibling.title)))
}
pub async fn decide(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<Decision>,
) -> Result<Json<Value>> {
    actor.owner()?;
    require(
        ["approve", "reject"].contains(&body.decision.as_str()),
        "决定不合法",
    )?;
    let inner = state.lock()?;
    let outcome = inner.store.transaction(|store| {
        let mut p = store.entity(&id)?;
        require(p.kind == "proposal", "此记录不是建议")?;
        if p.status != "pending" || p.version != body.expected_version {
            return Err(Error::Conflict("建议已经处理，请刷新".into()));
        }
        if body.decision == "reject" {
            p.status = "rejected".into();
            store.update(&p)?;
            store.audit("owner", "proposal.rejected", &id)?;
            return Ok(json!({"task_id": null}));
        }
        let action = p.properties["action"]
            .as_str()
            .unwrap_or("create_task")
            .to_string();
        require(
            body.directive_id.is_empty() || action == "create_task",
            "只有任务建议可以关联执行方向",
        )?;
        let source = store.entity(p.properties["source_id"].as_str().ok_or(Error::NotFound)?)?;
        // Imports are immutable payloads; other sources may have changed since.
        if source.kind != "agent_import"
            && source.version != p.properties["source_version"].as_i64().unwrap_or(0)
        {
            return Err(Error::Conflict("依据已更新，请拒绝旧建议并重新分析".into()));
        }
        let title = if body.title.trim().is_empty() {
            p.title.clone()
        } else {
            body.title.trim().into()
        };
        let mut outcome = json!({"task_id": null, "entity_id": null, "relation_id": null});
        match action.as_str() {
            "create_task" => {
                validate_text(&title, 120)?;
                if !body.directive_id.is_empty() {
                    require(
                        store.entity(&body.directive_id)?.kind == "directive",
                        "执行方向不合法",
                    )?;
                }
                let description = match text(&p, "body") {
                    "" => text(&p, "rationale"),
                    written => written,
                }
                .to_string();
                let task = store.insert_scoped(
                    "task",
                    &title,
                    &p.values,
                    &p.scenes,
                    json!({"body": description, "due_date": "", "directive_id": body.directive_id}),
                    "todo",
                )?;
                store.relate(&task.id, "derived_from", &p.id)?;
                if source.kind == "resource" {
                    store.relate(&task.id, "uses", &source.id)?;
                }
                if !body.directive_id.is_empty() {
                    store.relate(&task.id, "implements", &body.directive_id)?;
                }
                p.properties["task_id"] = json!(task.id);
                outcome["task_id"] = json!(task.id);
            }
            "create_entity" => {
                let spec = p.properties["entity"].clone();
                let kind = spec["kind"].as_str().unwrap_or("");
                require(
                    operations::AGENT_CREATABLE.contains(&kind),
                    "建议的实体类型不允许",
                )?;
                let entity = operations::insert_entity(
                    store,
                    kind,
                    &title,
                    &p.values,
                    &p.scenes,
                    spec["properties"].clone(),
                )?;
                store.relate(&entity.id, "derived_from", &p.id)?;
                store.audit("owner", "entity.created", &entity.id)?;
                p.properties["created_entity_id"] = json!(entity.id);
                outcome["entity_id"] = json!(entity.id);
            }
            "create_relation" => {
                let spec = p.properties["relation"].clone();
                let predicate = spec["predicate"].as_str().unwrap_or("");
                require(
                    ontology::predicate_def(predicate).is_some_and(|d| d.agent_proposable),
                    "建议的关系类型不允许",
                )?;
                let subject = resolve(store, &p, spec["subject"].as_str().unwrap_or(""))?;
                let object = resolve(store, &p, spec["object"].as_str().unwrap_or(""))?;
                let relation = store.relate(&subject, predicate, &object)?;
                store.audit("owner", "relation.created", &relation.id)?;
                p.properties["created_relation_id"] = json!(relation.id);
                outcome["relation_id"] = json!(relation.id);
            }
            _ => return Err(Error::Bad("建议动作不合法".into())),
        }
        p.status = "approved".into();
        store.update(&p)?;
        store.audit("owner", "proposal.approved", &id)?;
        Ok(outcome)
    })?;
    Ok(Json(outcome))
}
