//! PRP (personal resource planning): physical-world resources following the REA
//! ontology — item master (resource), locations, suppliers, purchase orders as
//! commitments, and immutable stock / cash events that fulfil them.
use crate::{
    error::{require, Error, Result},
    ontology::{self, is_date, validate_optional, validate_text, validate_values, Entity},
    store::{now, Store},
    Actor, AppState,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub const ITEM_TYPES: &[(&str, &str)] = &[("consumable", "消耗品"), ("asset", "资产")];
const MAX_QUANTITY: i64 = 1_000_000_000;
const MAX_CENTS: i64 = 1_000_000_000_000;

/// Empty ids are allowed; otherwise the id must name an entity of `kind`.
fn reference(store: &Store, id: &str, kind: &str, message: &str) -> Result<Option<Entity>> {
    if id.is_empty() {
        return Ok(None);
    }
    match store.entity(id) {
        Ok(entity) if entity.kind == kind => Ok(Some(entity)),
        Ok(_) | Err(Error::NotFound) => Err(Error::Bad(message.into())),
        Err(error) => Err(error),
    }
}
fn consumable() -> String {
    "consumable".into()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceProps {
    unit: String,
    #[serde(default)]
    location: String,
    minimum: i64,
    #[serde(default)]
    sku: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    supplier: String,
    #[serde(default)]
    supplier_id: String,
    #[serde(default)]
    unit_cost_cents: i64,
    #[serde(default = "consumable")]
    item_type: String,
    #[serde(default)]
    reorder_qty: i64,
    #[serde(default)]
    lead_time_days: i64,
    #[serde(default)]
    location_id: String,
    #[serde(default)]
    warranty_until: String,
    #[serde(default)]
    serial_no: String,
}
/// Item master record. `minimum` is the reorder point; quantity starts at 0 and
/// only changes through stock movements.
pub fn resource_properties(store: &Store, props: Value) -> Result<Value> {
    let p: ResourceProps =
        serde_json::from_value(props).map_err(|_| Error::Bad("资源字段不合法".into()))?;
    validate_text(&p.unit, 20)?;
    let place = reference(store, &p.location_id, "location", "库位不存在")?;
    let location = match (p.location.trim(), &place) {
        ("", Some(place)) => place.title.clone(),
        (text, _) => text.to_string(),
    };
    validate_text(&location, 120)?;
    require((0..=MAX_QUANTITY).contains(&p.minimum), "最低库存不合法")?;
    require(
        (0..=MAX_QUANTITY).contains(&p.reorder_qty),
        "建议补货量不合法",
    )?;
    require((0..=3650).contains(&p.lead_time_days), "交货周期不合法")?;
    require(
        (0..=MAX_CENTS).contains(&p.unit_cost_cents),
        "单位成本不合法",
    )?;
    require(
        ITEM_TYPES.iter().any(|(id, _)| *id == p.item_type),
        "物料类型不合法",
    )?;
    require(
        p.sku.chars().count() <= 80
            && p.category.chars().count() <= 80
            && p.supplier.chars().count() <= 120
            && p.serial_no.chars().count() <= 80,
        "资源 PRP 字段过长",
    )?;
    require(
        p.warranty_until.is_empty() || is_date(&p.warranty_until),
        "保修日期不合法",
    )?;
    let supplier = reference(store, &p.supplier_id, "supplier", "供应商不存在")?;
    let supplier_name = match (p.supplier.trim(), &supplier) {
        ("", Some(s)) => s.title.clone(),
        (text, _) => text.to_string(),
    };
    Ok(json!({
        "unit": p.unit, "location": location, "location_id": p.location_id,
        "minimum": p.minimum, "quantity": 0, "reorder_qty": p.reorder_qty, "lead_time_days": p.lead_time_days,
        "sku": p.sku, "category": p.category, "item_type": p.item_type,
        "supplier": supplier_name, "supplier_id": p.supplier_id, "unit_cost_cents": p.unit_cost_cents,
        "warranty_until": p.warranty_until, "serial_no": p.serial_no,
    }))
}
pub fn location_properties(store: &Store, props: Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LocationProps {
        #[serde(default)]
        code: String,
        #[serde(default)]
        parent_id: String,
        #[serde(default)]
        notes: String,
    }
    let p: LocationProps =
        serde_json::from_value(props).map_err(|_| Error::Bad("库位字段不合法".into()))?;
    require(
        p.code.chars().count() <= 40 && p.notes.chars().count() <= 1000,
        "库位字段过长",
    )?;
    reference(store, &p.parent_id, "location", "上级库位不存在")?;
    Ok(json!({"code": p.code, "parent_id": p.parent_id, "notes": p.notes}))
}
pub fn supplier_properties(props: Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SupplierProps {
        #[serde(default)]
        contact: String,
        #[serde(default)]
        tax_id: String,
        #[serde(default)]
        notes: String,
    }
    let p: SupplierProps =
        serde_json::from_value(props).map_err(|_| Error::Bad("供应商字段不合法".into()))?;
    require(
        p.contact.chars().count() <= 200
            && p.tax_id.chars().count() <= 80
            && p.notes.chars().count() <= 1000,
        "供应商字段过长",
    )?;
    Ok(json!({"contact": p.contact, "tax_id": p.tax_id, "notes": p.notes}))
}
pub fn order_properties(store: &Store, props: Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct OrderProps {
        #[serde(default)]
        supplier_id: String,
        #[serde(default)]
        expected_date: String,
        #[serde(default)]
        notes: String,
    }
    let p: OrderProps =
        serde_json::from_value(props).map_err(|_| Error::Bad("采购订单字段不合法".into()))?;
    let supplier = reference(store, &p.supplier_id, "supplier", "供应商不存在")?;
    require(
        p.expected_date.is_empty() || is_date(&p.expected_date),
        "日期格式不合法",
    )?;
    require(p.notes.chars().count() <= 4000, "订单备注过长")?;
    Ok(json!({
        "supplier_id": p.supplier_id, "supplier": supplier.map(|s| s.title).unwrap_or_default(),
        "expected_date": p.expected_date, "notes": p.notes,
        "total_cents": 0, "paid_cents": 0, "ordered_at": "", "received_at": "", "closed_at": "",
    }))
}
/// Order lines may only be added while the order is a draft ("active" is the
/// pre-0.4 status of a draft order).
pub fn line_properties(store: &Store, props: Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LineProps {
        purchase_order_id: String,
        resource_id: String,
        quantity: i64,
        unit_cost_cents: i64,
    }
    let p: LineProps =
        serde_json::from_value(props).map_err(|_| Error::Bad("采购订单行字段不合法".into()))?;
    let order = reference(
        store,
        &p.purchase_order_id,
        "purchase_order",
        "采购订单不存在",
    )?
    .ok_or(Error::Bad("采购订单不存在".into()))?;
    require(is_draft(&order), "只有草稿状态的采购订单可以添加订单行")?;
    let resource = reference(store, &p.resource_id, "resource", "资源不存在")?
        .ok_or(Error::Bad("资源不存在".into()))?;
    require(
        (1..=MAX_QUANTITY).contains(&p.quantity) && (0..=MAX_CENTS).contains(&p.unit_cost_cents),
        "订单行数值不合法",
    )?;
    let total = p
        .quantity
        .checked_mul(p.unit_cost_cents)
        .filter(|total| *total <= MAX_CENTS)
        .ok_or(Error::Bad("订单金额超出范围".into()))?;
    Ok(json!({
        "purchase_order_id": p.purchase_order_id, "resource_id": p.resource_id,
        "resource_title": resource.title, "unit": resource.properties["unit"],
        "quantity": p.quantity, "received_quantity": 0,
        "unit_cost_cents": p.unit_cost_cents, "line_total_cents": total,
    }))
}
fn is_draft(order: &Entity) -> bool {
    ["draft", "active"].contains(&order.status.as_str())
}
fn cents(entity: &Entity, key: &str) -> i64 {
    entity.properties[key].as_i64().unwrap_or(0)
}
/// Keep the order total equal to the sum of its lines.
pub fn add_line_total(store: &Store, order_id: &str, line_total: i64) -> Result<()> {
    let mut order = store.entity(order_id)?;
    let total = cents(&order, "total_cents")
        .checked_add(line_total)
        .filter(|total| *total <= MAX_CENTS)
        .ok_or(Error::Bad("订单金额超出范围".into()))?;
    order.properties["total_cents"] = json!(total);
    store.update(&order)?;
    Ok(())
}
pub fn order_lines(store: &Store, order_id: &str) -> Result<Vec<Entity>> {
    Ok(store
        .entities_of(&["purchase_order_line"])?
        .into_iter()
        .filter(|line| line.properties["purchase_order_id"] == order_id)
        .collect())
}
/// Post an immutable stock event and update the on-hand balance.
pub fn post_movement(
    store: &Store,
    resource: &Entity,
    delta: i64,
    reason: &str,
    movement_type: &str,
    unit_cost_cents: Option<i64>,
    extra: Value,
) -> Result<Entity> {
    let quantity = resource.properties["quantity"]
        .as_i64()
        .ok_or(Error::Crypto)?;
    let next = quantity
        .checked_add(delta)
        .ok_or(Error::Bad("数量超出范围".into()))?;
    require((0..=MAX_QUANTITY).contains(&next), "库存不足或超过允许范围")?;
    let mut updated = resource.clone();
    updated.properties["quantity"] = json!(next);
    if let Some(cost) = unit_cost_cents.filter(|cost| *cost > 0) {
        updated.properties["unit_cost_cents"] = json!(cost);
    }
    store.update(&updated)?;
    let mut props = json!({
        "resource_id": resource.id, "delta": delta, "balance": next,
        "unit": resource.properties["unit"], "movement_type": movement_type,
    });
    if let (Some(target), Some(extra)) = (props.as_object_mut(), extra.as_object()) {
        target.extend(extra.clone());
    }
    let entry = store.insert_scoped(
        "stock_movement",
        reason,
        &resource.values,
        &ontology::scenes_for("stock_movement", &resource.scenes)?,
        props,
        "posted",
    )?;
    store.relate(&entry.id, "records", &resource.id)?;
    Ok(entry)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Movement {
    pub delta: i64,
    pub reason: String,
    pub expected_version: i64,
    #[serde(default)]
    pub movement_type: String,
}
pub async fn movement(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<Movement>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.reason, 300)?;
    require(
        body.delta != 0 && (-MAX_QUANTITY..=MAX_QUANTITY).contains(&body.delta),
        "变动数量不合法",
    )?;
    let movement_type = match body.movement_type.as_str() {
        "" if body.delta > 0 => "receipt",
        "" => "issue",
        other => other,
    };
    require(
        match movement_type {
            "receipt" => body.delta > 0,
            "issue" => body.delta < 0,
            "adjustment" => true,
            _ => false,
        },
        "变动类型与数量方向不一致",
    )?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let resource = store.entity(&id)?;
        require(resource.kind == "resource", "此记录不是资源")?;
        if resource.version != body.expected_version {
            return Err(Error::Conflict("库存已有更新，请刷新后重试".into()));
        }
        let entry = post_movement(
            store,
            &resource,
            body.delta,
            &body.reason,
            movement_type,
            None,
            json!({}),
        )?;
        store.audit("owner", "stock.posted", &entry.id)
    })?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cash {
    pub title: String,
    pub direction: String,
    pub amount_cents: i64,
    pub category: String,
    pub date: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub scenes: Vec<String>,
}
pub async fn cash(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Cash>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.title, 120)?;
    validate_text(&body.category, 60)?;
    validate_values(&body.values)?;
    let scenes = ontology::scenes_for("cash_entry", &body.scenes)?;
    require(
        ["income", "expense"].contains(&body.direction.as_str()),
        "收支方向不合法",
    )?;
    require((1..=MAX_CENTS).contains(&body.amount_cents), "金额不合法")?;
    require(is_date(&body.date), "日期格式不合法")?;
    let inner = state.lock()?;
    let entry = inner.store.transaction(|store| {
        let entry = store.insert_scoped(
            "cash_entry",
            &body.title,
            &body.values,
            &scenes,
            json!({"direction":body.direction,"amount_cents":body.amount_cents,"category":body.category,"date":body.date,"currency":"CNY"}),
            "posted",
        )?;
        store.audit("owner", "cash.posted", &entry.id)?;
        Ok(entry)
    })?;
    Ok(Json(json!(entry)))
}
fn current_order(store: &Store, id: &str, expected_version: i64) -> Result<Entity> {
    let order = store.entity(id)?;
    require(order.kind == "purchase_order", "此记录不是采购订单")?;
    if order.version != expected_version {
        return Err(Error::Conflict("采购订单已更新，请刷新后重试".into()));
    }
    Ok(order)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderStatus {
    pub status: String,
    pub expected_version: i64,
}
/// draft → ordered → (partially_received →) received → closed; cancel only
/// before anything was received or paid.
pub async fn order_status(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<OrderStatus>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut order = current_order(store, &id, body.expected_version)?;
        let lines = order_lines(store, &order.id)?;
        let current = order.status.as_str();
        match body.status.as_str() {
            "ordered" => {
                require(is_draft(&order), "只有草稿可以下单")?;
                require(!lines.is_empty(), "请先添加订单行")?;
                order.properties["ordered_at"] = json!(now());
            }
            "cancelled" => {
                require(
                    is_draft(&order) || current == "ordered",
                    "已收货的采购订单不能取消",
                )?;
                require(
                    cents(&order, "paid_cents") == 0
                        && lines.iter().all(|l| cents(l, "received_quantity") == 0),
                    "已收货或已付款的采购订单不能取消",
                )?;
            }
            "closed" => {
                require(
                    ["ordered", "partially_received", "received"].contains(&current),
                    "只有已下单的采购订单可以完结",
                )?;
                order.properties["closed_at"] = json!(now());
            }
            _ => return Err(Error::Bad("采购订单状态不合法".into())),
        }
        order.status = body.status.clone();
        store.update(&order)?;
        store.audit("owner", "purchase.status_changed", &order.id)
    })?;
    Ok(Json(json!({"ok": true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiveLine {
    pub line_id: String,
    pub quantity: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receive {
    pub lines: Vec<ReceiveLine>,
    #[serde(default)]
    pub note: String,
    pub expected_version: i64,
}
/// Goods receipt against order lines: each line posts a receipt movement.
pub async fn receive(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<Receive>,
) -> Result<Json<Value>> {
    actor.owner()?;
    require(
        !body.lines.is_empty() && body.lines.len() <= 100,
        "请选择收货的订单行",
    )?;
    validate_optional(&body.note, 300, "收货备注过长")?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut order = current_order(store, &id, body.expected_version)?;
        require(
            ["ordered", "partially_received"].contains(&order.status.as_str()),
            "只有已下单的采购订单可以收货",
        )?;
        for (index, item) in body.lines.iter().enumerate() {
            require(
                body.lines[..index]
                    .iter()
                    .all(|o| o.line_id != item.line_id),
                "订单行重复",
            )?;
            let mut line = store.entity(&item.line_id)?;
            require(
                line.kind == "purchase_order_line"
                    && line.properties["purchase_order_id"] == order.id,
                "订单行不属于此采购订单",
            )?;
            let ordered = cents(&line, "quantity");
            let received = cents(&line, "received_quantity");
            require(
                item.quantity >= 1 && received + item.quantity <= ordered,
                "收货数量超过未收数量",
            )?;
            let resource = store.entity(line.properties["resource_id"].as_str().unwrap_or(""))?;
            require(resource.kind == "resource", "订单行的资源不存在")?;
            let reason = if body.note.trim().is_empty() {
                format!("采购收货：{}", order.title)
            } else {
                body.note.trim().to_string()
            };
            let entry = post_movement(
                store,
                &resource,
                item.quantity,
                &reason,
                "receipt",
                Some(cents(&line, "unit_cost_cents")),
                json!({"purchase_order_id": order.id, "purchase_order_line_id": line.id}),
            )?;
            store.relate(&entry.id, "receives", &line.id)?;
            line.properties["received_quantity"] = json!(received + item.quantity);
            if received + item.quantity == ordered {
                line.status = "received".into();
            }
            store.update(&line)?;
        }
        let complete = order_lines(store, &order.id)?
            .iter()
            .all(|l| cents(l, "received_quantity") >= cents(l, "quantity"));
        order.status = if complete {
            "received"
        } else {
            "partially_received"
        }
        .into();
        order.properties["received_at"] = json!(now());
        store.update(&order)?;
        store.audit("owner", "purchase.received", &order.id)
    })?;
    Ok(Json(json!({"ok": true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pay {
    pub amount_cents: i64,
    pub date: String,
    pub expected_version: i64,
    #[serde(default)]
    pub title: String,
}
/// Payment for an order: a cash expense linked by `pays`, capped at the unpaid amount.
pub async fn pay(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<Pay>,
) -> Result<Json<Value>> {
    actor.owner()?;
    require(is_date(&body.date), "日期格式不合法")?;
    validate_optional(&body.title, 120, "付款摘要过长")?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut order = current_order(store, &id, body.expected_version)?;
        require(
            ["ordered", "partially_received", "received", "closed"]
                .contains(&order.status.as_str()),
            "请先下单再付款",
        )?;
        let outstanding = cents(&order, "total_cents") - cents(&order, "paid_cents");
        require(
            body.amount_cents >= 1 && body.amount_cents <= outstanding,
            "付款金额超过未付金额",
        )?;
        let title = if body.title.trim().is_empty() {
            format!("采购付款：{}", order.title)
        } else {
            body.title.trim().to_string()
        };
        let entry = store.insert_scoped(
            "cash_entry",
            &title,
            &order.values,
            &ontology::scenes_for("cash_entry", &order.scenes)?,
            json!({"direction":"expense","amount_cents":body.amount_cents,"category":"采购","date":body.date,"currency":"CNY","purchase_order_id":order.id}),
            "posted",
        )?;
        store.relate(&entry.id, "pays", &order.id)?;
        order.properties["paid_cents"] = json!(cents(&order, "paid_cents") + body.amount_cents);
        store.update(&order)?;
        store.audit("owner", "purchase.paid", &order.id)
    })?;
    Ok(Json(json!({"ok": true})))
}
