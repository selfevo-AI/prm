use crate::{
    agents, erp,
    error::{require, Error, Result},
    ontology::{self, is_date, validate_text, validate_values, Entity, EntityInput},
    relationships,
    store::Store,
    Actor, AppState,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Kinds the owner may create through POST /api/entities.
pub const OWNER_CREATABLE: &[&str] = &[
    "resource",
    "location",
    "supplier",
    "purchase_order",
    "purchase_order_line",
    "task",
    "note",
    "channel",
    "person",
];
/// Kinds an agent may suggest as create_entity candidates; never ledgers, vault or self.
pub const AGENT_CREATABLE: &[&str] = &["resource", "location", "supplier", "note", "person"];

pub async fn workspace(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    Ok(Json(
        json!({"entities":inner.store.entities()?,"relations":inner.store.relations()?,"audit":inner.store.audits()?,"ontology":ontology::schema(),"analysis_mode":"offline-rules"}),
    ))
}
/// Validate kind-specific properties; returns the normalized properties and the initial status.
pub fn normalize(store: &Store, kind: &str, properties: Value) -> Result<(Value, &'static str)> {
    match kind {
        "resource" => Ok((erp::resource_properties(store, properties)?, "active")),
        "location" => Ok((erp::location_properties(store, properties)?, "active")),
        "supplier" => Ok((erp::supplier_properties(properties)?, "active")),
        "purchase_order" => Ok((erp::order_properties(store, properties)?, "draft")),
        "purchase_order_line" => Ok((erp::line_properties(store, properties)?, "open")),
        "person" => Ok((relationships::person_properties(properties)?, "active")),
        "task" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct TaskProps {
                #[serde(default)]
                body: String,
                #[serde(default)]
                due_date: String,
                #[serde(default)]
                directive_id: String,
            }
            let p: TaskProps = serde_json::from_value(properties)
                .map_err(|_| Error::Bad("任务字段不合法".into()))?;
            require(p.body.chars().count() <= 4000, "任务说明过长")?;
            require(
                p.due_date.is_empty() || is_date(&p.due_date),
                "日期格式不合法",
            )?;
            if !p.directive_id.is_empty() {
                require(
                    store.entity(&p.directive_id)?.kind == "directive",
                    "执行方向不合法",
                )?;
            }
            Ok((
                json!({"body":p.body,"due_date":p.due_date,"directive_id":p.directive_id}),
                "todo",
            ))
        }
        "note" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct NoteProps {
                body: String,
            }
            let p: NoteProps = serde_json::from_value(properties)
                .map_err(|_| Error::Bad("知识字段不合法".into()))?;
            validate_text(&p.body, 12000)?;
            Ok((json!({"body":p.body}), "active"))
        }
        "channel" => {
            require(properties == json!({}), "会话不接受额外字段")?;
            Ok((json!({}), "active"))
        }
        _ => Err(Error::Forbidden),
    }
}
/// Insert a validated entity together with the relations implied by its properties.
pub fn insert_entity(
    store: &Store,
    kind: &str,
    title: &str,
    values: &[String],
    scenes: &[String],
    properties: Value,
) -> Result<Entity> {
    validate_text(title, 120)?;
    validate_values(values)?;
    let scenes = ontology::scenes_for(kind, scenes)?;
    let (props, status) = normalize(store, kind, properties)?;
    let entity = store.insert_scoped(kind, title, values, &scenes, props, status)?;
    let text = |key: &str| {
        entity.properties[key]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    match kind {
        "task" => {
            if let Some(directive) = text("directive_id") {
                store.relate(&entity.id, "implements", &directive)?;
            }
        }
        "resource" => {
            if let Some(supplier) = text("supplier_id") {
                store.relate(&supplier, "supplies", &entity.id)?;
            }
            if let Some(location) = text("location_id") {
                store.relate(&entity.id, "stored_at", &location)?;
            }
        }
        "location" => {
            if let Some(parent) = text("parent_id") {
                store.relate(&entity.id, "located_in", &parent)?;
            }
        }
        "purchase_order" => {
            if let Some(supplier) = text("supplier_id") {
                store.relate(&supplier, "supplies", &entity.id)?;
            }
        }
        "purchase_order_line" => {
            let order = text("purchase_order_id").ok_or(Error::NotFound)?;
            let resource = text("resource_id").ok_or(Error::NotFound)?;
            store.relate(&order, "contains", &entity.id)?;
            store.relate(&entity.id, "orders", &resource)?;
            erp::add_line_total(
                store,
                &order,
                entity.properties["line_total_cents"].as_i64().unwrap_or(0),
            )?;
        }
        _ => {}
    }
    Ok(entity)
}
pub async fn create(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<EntityInput>,
) -> Result<Json<Value>> {
    actor.owner()?;
    if !OWNER_CREATABLE.contains(&body.kind.as_str()) {
        return Err(Error::Forbidden);
    }
    let inner = state.lock()?;
    let entity = inner.store.transaction(|store| {
        let entity = insert_entity(
            store,
            &body.kind,
            &body.title,
            &body.values,
            &body.scenes,
            body.properties,
        )?;
        store.audit("owner", "entity.created", &entity.id)?;
        Ok(entity)
    })?;
    Ok(Json(json!(entity)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub status: String,
    pub expected_version: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revoke {
    pub expected_version: i64,
}

pub async fn revoke_directive(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<Revoke>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut directive = store.entity(&id)?;
        require(directive.kind == "directive", "此记录不是执行方向")?;
        if directive.version != body.expected_version || directive.status != "active" {
            return Err(Error::Conflict("执行方向已更新或撤回".into()));
        }
        directive.status = "revoked".into();
        store.update(&directive)?;
        store.audit("owner", "directive.revoked", &id)
    })?;
    Ok(Json(json!({"ok": true})))
}
pub async fn task_status(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<Status>,
) -> Result<Json<Value>> {
    actor.owner()?;
    require(
        ["todo", "doing", "done", "cancelled"].contains(&body.status.as_str()),
        "任务状态不合法",
    )?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut entity = store.entity(&id)?;
        require(entity.kind == "task", "此记录不是任务")?;
        if entity.version != body.expected_version {
            return Err(Error::Conflict("任务已有更新".into()));
        }
        entity.status = body.status;
        store.update(&entity)?;
        store.audit("owner", "task.status_changed", &id)
    })?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}
pub async fn relate(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Link>,
) -> Result<Json<Value>> {
    actor.owner()?;
    require(
        ontology::predicate_def(&body.predicate).is_some_and(|p| p.owner_settable),
        "系统来源关系只能随业务操作建立",
    )?;
    let inner = state.lock()?;
    let relation = inner.store.transaction(|store| {
        let relation = store.relate(&body.subject, &body.predicate, &body.object)?;
        store.audit("owner", "relation.created", &relation.id)?;
        Ok(relation)
    })?;
    Ok(Json(json!(relation)))
}
pub async fn export(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    Ok(Json(
        json!({"format":"prm-operations-v2","exported_at":crate::store::now(),"entities":inner.store.entities()?,"relations":inner.store.relations()?,"audit":inner.store.audits()?,"agent_connections":agents::credentials::connections(&inner.store)?,"ontology":ontology::schema()}),
    ))
}
fn seed(store: &Store) -> Result<()> {
    let need = vec!["need".into()];
    let desire = vec!["desire".into()];
    let room = store.insert(
        "location",
        "储物间 A-02",
        &[],
        json!({"code":"A-02","parent_id":"","notes":"演示库位"}),
        "active",
    )?;
    let shop = store.insert(
        "supplier",
        "家居耗材店（演示）",
        &[],
        json!({"contact":"","tax_id":"","notes":"演示供应商"}),
        "active",
    )?;
    let resource = store.insert(
        "resource",
        "滤芯 / 家用净水器",
        &need,
        json!({"unit":"个","location":room.title,"location_id":room.id,"quantity":1,"minimum":2,"reorder_qty":2,"lead_time_days":3,"item_type":"consumable","sku":"FILTER-01","category":"家居耗材","supplier":shop.title,"supplier_id":shop.id,"unit_cost_cents":6900,"warranty_until":"","serial_no":""}),
        "active",
    )?;
    store.relate(&resource.id, "stored_at", &room.id)?;
    store.relate(&shop.id, "supplies", &resource.id)?;
    let entry = store.insert(
        "stock_movement",
        "期初库存（演示）",
        &need,
        json!({"resource_id":resource.id,"delta":1,"balance":1,"unit":"个","movement_type":"opening"}),
        "posted",
    )?;
    store.relate(&entry.id, "records", &resource.id)?;
    let laptop = store.insert_scoped(
        "resource",
        "工作笔记本电脑",
        &desire,
        &["work".into()],
        json!({"unit":"台","location":"书房","location_id":"","quantity":1,"minimum":0,"reorder_qty":0,"lead_time_days":0,"item_type":"asset","sku":"","category":"电子设备","supplier":"","supplier_id":"","unit_cost_cents":0,"warranty_until":"","serial_no":""}),
        "active",
    )?;
    let entry = store.insert(
        "stock_movement",
        "期初库存（演示）",
        &desire,
        json!({"resource_id":laptop.id,"delta":1,"balance":1,"unit":"台","movement_type":"opening"}),
        "posted",
    )?;
    store.relate(&entry.id, "records", &laptop.id)?;
    store.insert("note","三世界经济模型 · 阅读摘记",&["existence".into()],json!({"body":"物理、数字、思维是相互交叉的活动维度。需要、欲望、存在是独立的评价维度。资源增加并不自动意味着生活改善。"}),"active")?;
    let task=store.insert_scoped("task","整理本周学习资料",&desire,&["learning".into()],json!({"body":"检查资料来源，保留自己的阅读笔记。","due_date":chrono::Local::now().date_naive().to_string(),"directive_id":""}),"todo")?;
    store.relate(&task.id, "uses", &laptop.id)?;
    store.insert_scoped("cash_entry","图书采购（演示）",&desire,&["learning".into(),"finance".into()],json!({"direction":"expense","amount_cents":12800,"category":"学习","date":chrono::Local::now().date_naive().to_string(),"currency":"CNY"}),"posted")?;
    store.audit("owner", "demo.created", "workspace")
}
pub async fn demo(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    let exists: bool = inner.store.db.query_row(
        "SELECT EXISTS(SELECT 1 FROM audit WHERE action='demo.created')",
        &[],
        |r| r.get(0),
    )?;
    if exists {
        return Err(Error::Conflict("演示数据已经添加过".into()));
    }
    inner.store.transaction(seed)?;
    Ok(Json(json!({"ok":true})))
}
