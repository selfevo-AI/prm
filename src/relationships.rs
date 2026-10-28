//! Relations with other people: the personal-CRM counterpart of the PRP side.
//! A relationship is a reified relation between the self and a person, with
//! owner-rated closeness (IOS) and satisfaction (RAS) and an interaction ledger.
use crate::{
    error::{require, Error, Result},
    ontology::{self, is_date, validate_optional, validate_text, validate_values, Entity},
    self_model,
    store::Store,
    Actor, AppState,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};

/// (id, label, default scene)
pub const RELATIONSHIP_TYPES: &[(&str, &str, &str)] = &[
    ("family", "家人", "emotion"),
    ("partner", "伴侣", "emotion"),
    ("friend", "朋友", "social"),
    ("colleague", "同事", "work"),
    ("mentor", "导师", "learning"),
    ("client", "客户", "work"),
    ("community", "社群", "social"),
    ("other", "其他", "social"),
];
pub const INTERACTION_CHANNELS: &[(&str, &str)] = &[
    ("meet", "见面"),
    ("call", "通话"),
    ("message", "消息"),
    ("online", "线上活动"),
    ("other", "其他"),
];
pub const RELATIONSHIP_STATUSES: &[&str] = &["active", "paused", "ended"];

pub fn person_properties(props: Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PersonProps {
        #[serde(default)]
        notes: String,
        #[serde(default)]
        agent_visible: bool,
    }
    let p: PersonProps =
        serde_json::from_value(props).map_err(|_| Error::Bad("人物字段不合法".into()))?;
    validate_optional(&p.notes, 2000, "人物备注过长")?;
    Ok(json!({"notes": p.notes, "agent_visible": p.agent_visible}))
}
fn relationship_type(id: &str) -> Result<&'static (&'static str, &'static str, &'static str)> {
    RELATIONSHIP_TYPES
        .iter()
        .find(|(t, ..)| *t == id)
        .ok_or(Error::Bad("关系类型不合法".into()))
}
fn check_measures(
    closeness: i64,
    satisfaction: Option<f64>,
    interval: i64,
    notes: &str,
) -> Result<()> {
    require((1..=7).contains(&closeness), "亲密度（IOS）应为 1–7")?;
    require(
        satisfaction.is_none_or(|s| s.is_finite() && (1.0..=5.0).contains(&s)),
        "关系满意度（RAS）应为 1–5",
    )?;
    require((0..=3650).contains(&interval), "联系节奏应为 0–3650 天")?;
    validate_optional(notes, 2000, "关系备注过长")
}
fn default_closeness() -> i64 {
    4
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipInput {
    #[serde(default)]
    pub person_id: String,
    #[serde(default)]
    pub person_name: String,
    #[serde(rename = "type")]
    pub relationship_type: String,
    #[serde(default = "default_closeness")]
    pub closeness: i64,
    #[serde(default)]
    pub satisfaction: Option<f64>,
    #[serde(default)]
    pub contact_interval_days: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub agent_visible: bool,
}
pub async fn create(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<RelationshipInput>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let (type_id, type_label, default_scene) = *relationship_type(&body.relationship_type)?;
    check_measures(
        body.closeness,
        body.satisfaction,
        body.contact_interval_days,
        &body.notes,
    )?;
    validate_values(&body.values)?;
    ontology::validate_scenes(&body.scenes)?;
    let scenes = if body.scenes.is_empty() {
        vec![default_scene.to_string()]
    } else {
        body.scenes.clone()
    };
    let inner = state.lock()?;
    let relationship = inner.store.transaction(|store| {
        let person = if body.person_id.is_empty() {
            validate_text(&body.person_name, 120)?;
            store.insert_scoped(
                "person",
                &body.person_name,
                &[],
                &scenes,
                json!({"notes": "", "agent_visible": body.agent_visible}),
                "active",
            )?
        } else {
            let person = store.entity(&body.person_id)?;
            require(person.kind == "person", "人物不存在")?;
            person
        };
        let profile = self_model::ensure_self_profile(store)?;
        let entity = store.insert_scoped(
            "relationship",
            &format!("{} · {}", person.title, type_label),
            &body.values,
            &scenes,
            json!({
                "type": type_id, "person_id": person.id, "person_name": person.title,
                "closeness": body.closeness, "satisfaction": body.satisfaction,
                "contact_interval_days": body.contact_interval_days, "last_contact_at": "",
                "notes": body.notes, "agent_visible": body.agent_visible,
            }),
            "active",
        )?;
        store.relate(&entity.id, "involves", &person.id)?;
        store.relate(&entity.id, "relates_to_self", &profile.id)?;
        store.audit("owner", "relationship.created", &entity.id)?;
        Ok(entity)
    })?;
    Ok(Json(json!(relationship)))
}
fn current(store: &Store, id: &str, expected_version: i64) -> Result<Entity> {
    let relationship = store.entity(id)?;
    require(relationship.kind == "relationship", "此记录不是人际关系")?;
    if relationship.version != expected_version {
        return Err(Error::Conflict("关系已有更新，请刷新后重试".into()));
    }
    Ok(relationship)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipUpdate {
    pub closeness: i64,
    #[serde(default)]
    pub satisfaction: Option<f64>,
    pub contact_interval_days: i64,
    pub status: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    pub expected_version: i64,
}
pub async fn update(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<RelationshipUpdate>,
) -> Result<Json<Value>> {
    actor.owner()?;
    check_measures(
        body.closeness,
        body.satisfaction,
        body.contact_interval_days,
        &body.notes,
    )?;
    require(
        RELATIONSHIP_STATUSES.contains(&body.status.as_str()),
        "关系状态不合法",
    )?;
    ontology::validate_scenes(&body.scenes)?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut relationship = current(store, &id, body.expected_version)?;
        relationship.properties["closeness"] = json!(body.closeness);
        relationship.properties["satisfaction"] = json!(body.satisfaction);
        relationship.properties["contact_interval_days"] = json!(body.contact_interval_days);
        relationship.properties["notes"] = json!(body.notes);
        relationship.status = body.status.clone();
        if !body.scenes.is_empty() {
            relationship.scenes = body.scenes.clone();
        }
        store.update(&relationship)?;
        store.audit("owner", "relationship.updated", &id)
    })?;
    Ok(Json(json!({"ok": true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionInput {
    pub date: String,
    pub channel: String,
    #[serde(default)]
    pub valence: i64,
    #[serde(default)]
    pub notes: String,
    pub expected_version: i64,
}
/// Append an interaction event and advance the relationship's last contact.
pub async fn interaction(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<InteractionInput>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let date = NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| Error::Bad("日期格式不合法".into()))?;
    require(
        date <= chrono::Local::now().date_naive() + chrono::Days::new(1),
        "互动日期不能晚于今天",
    )?;
    let channel = INTERACTION_CHANNELS
        .iter()
        .find(|(c, _)| *c == body.channel)
        .ok_or(Error::Bad("互动方式不合法".into()))?;
    require((-2..=2).contains(&body.valence), "情绪体验应为 -2 至 2")?;
    validate_optional(&body.notes, 2000, "互动备注过长")?;
    let inner = state.lock()?;
    let entry = inner.store.transaction(|store| {
        let mut relationship = current(store, &id, body.expected_version)?;
        let person_name = relationship.properties["person_name"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let entry = store.insert_scoped(
            "interaction",
            &format!("{} · {}", channel.1, person_name),
            &relationship.values,
            &relationship.scenes,
            json!({
                "relationship_id": relationship.id, "person_id": relationship.properties["person_id"],
                "person_name": person_name, "date": body.date, "channel": body.channel,
                "valence": body.valence, "notes": body.notes,
            }),
            "posted",
        )?;
        store.relate(&entry.id, "occurs_in", &relationship.id)?;
        let last = relationship.properties["last_contact_at"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if last.is_empty() || last.as_str() < body.date.as_str() {
            relationship.properties["last_contact_at"] = json!(body.date);
        }
        store.update(&relationship)?;
        store.audit("owner", "relationship.interaction_recorded", &entry.id)?;
        Ok(entry)
    })?;
    Ok(Json(json!(entry)))
}
/// Days since the last contact when the owner's contact rhythm is exceeded.
pub fn contact_due(relationship: &Entity, today: NaiveDate) -> Option<i64> {
    if relationship.kind != "relationship" || relationship.status != "active" {
        return None;
    }
    let interval = relationship.properties["contact_interval_days"]
        .as_i64()
        .filter(|days| *days > 0)?;
    let last = relationship.properties["last_contact_at"]
        .as_str()
        .filter(|date| is_date(date))
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .or_else(|| {
            chrono::DateTime::parse_from_rfc3339(&relationship.created_at)
                .ok()
                .map(|at| at.date_naive())
        })?;
    let days = (today - last).num_days();
    (days >= interval).then_some(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relationship(last: &str, interval: i64, status: &str) -> Entity {
        Entity {
            id: "r".into(),
            kind: "relationship".into(),
            title: "李华 · 朋友".into(),
            worlds: vec!["self".into()],
            values: vec![],
            scenes: vec!["social".into()],
            properties: json!({"last_contact_at": last, "contact_interval_days": interval}),
            status: status.into(),
            version: 1,
            created_at: "2026-01-01T00:00:00+00:00".into(),
            updated_at: "2026-01-01T00:00:00+00:00".into(),
        }
    }

    #[test]
    fn contact_rhythm_is_due_only_when_exceeded() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(
            contact_due(&relationship("2026-02-01", 14, "active"), today),
            Some(28)
        );
        assert_eq!(
            contact_due(&relationship("2026-02-25", 14, "active"), today),
            None
        );
        assert_eq!(
            contact_due(&relationship("2026-02-01", 0, "active"), today),
            None
        );
        assert_eq!(
            contact_due(&relationship("2026-02-01", 14, "paused"), today),
            None
        );
        assert_eq!(
            contact_due(&relationship("", 30, "active"), today),
            Some(59)
        );
    }
}
