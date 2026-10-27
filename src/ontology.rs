//! Ontology registry: the single source of truth for entity classes, relation
//! predicates and the classification facets (worlds, value axes, life scenes).
//! SQL constraints, validation and the schema shown to the UI are derived from it.
use crate::error::{require, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Entity {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub worlds: Vec<String>,
    pub values: Vec<String>,
    #[serde(default)]
    pub scenes: Vec<String>,
    pub properties: Value,
    pub status: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityInput {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default = "empty_object")]
    pub properties: Value,
}
pub fn empty_object() -> Value {
    json!({})
}

/// Where a thing exists. Multi-valued and derived from the class.
pub const WORLDS: &[(&str, &str)] = &[
    ("self", "自我世界"),
    ("mental", "思维世界"),
    ("digital", "数字世界"),
    ("physical", "物理世界"),
];
/// How the owner values a thing (from the source paper); independent of worlds.
pub const VALUE_AXES: &[(&str, &str)] = &[("need", "需要"), ("desire", "欲望"), ("existence", "存在")];
/// Which part of life a thing belongs to; independent of worlds and value axes.
pub const SCENES: &[(&str, &str, &str)] = &[
    ("self", "自我", "自我认知、自我关系与个人原则"),
    ("emotion", "情感", "亲密关系、家庭与情绪体验"),
    ("life", "生活", "居家、日常事务与物品"),
    ("work", "工作", "职业、项目与同事协作"),
    ("health", "健康", "身体、睡眠、运动与照护"),
    ("learning", "学习", "知识、技能与成长"),
    ("finance", "财务", "收支、采购与资金安排"),
    ("social", "社交", "朋友、社群与公共参与"),
];
/// Upper-level categories, following the REA (Resource-Event-Agent) accounting
/// ontology extended with commitments, reified relations and observations.
pub const CATEGORIES: &[(&str, &str, &str)] = &[
    ("agent", "主体", "能够行动或承担关系的一方（REA：Agent）"),
    ("resource", "资源", "可被使用、维护或消耗的对象（REA：Resource）"),
    ("event", "事件", "已经发生、只追加不改写的记录（REA：Event）"),
    ("commitment", "承诺", "尚未兑现的计划、订单与待决建议（REA：Commitment）"),
    ("relation", "关系", "被具体化的关系：与自我、与他人、与身份"),
    ("observation", "观察", "本人记录的生理与心理指标，不是诊断"),
];
/// Relation layers: R = Resource and Relation, both with the self and the outside.
pub const LAYERS: &[(&str, &str, &str)] = &[
    ("self", "自我关系", "本人与自我：描述、测量、支持与影响"),
    ("external", "外部关系", "本人与他人、组织之间的关系与互动"),
    ("erp", "资源关系", "物料、库位、供应商、订单、收货与付款"),
    ("digital", "数字关系", "任务、知识、执行方向与会话之间的关联"),
    ("provenance", "来源追溯", "建议、导入与运行的来源，仅由系统建立"),
];

pub struct ClassDef {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub worlds: &'static [&'static str],
    /// Default scenes when the owner does not choose any.
    pub scenes: &'static [&'static str],
    pub authority: &'static str,
}
pub struct Signature {
    pub domain: &'static [&'static str],
    pub range: &'static [&'static str],
}
pub struct PredicateDef {
    pub id: &'static str,
    pub label: &'static str,
    pub layer: &'static str,
    /// The owner may create it through POST /api/relations.
    pub owner_settable: bool,
    /// An agent may suggest it as a create_relation candidate.
    pub agent_proposable: bool,
    pub signatures: &'static [Signature],
}

const fn class(
    id: &'static str,
    label: &'static str,
    category: &'static str,
    worlds: &'static [&'static str],
    scenes: &'static [&'static str],
    authority: &'static str,
) -> ClassDef {
    ClassDef {
        id,
        label,
        category,
        worlds,
        scenes,
        authority,
    }
}
const fn sig(domain: &'static [&'static str], range: &'static [&'static str]) -> Signature {
    Signature { domain, range }
}

/// Classes stored in the business `entities` table. The mental vault classes are
/// deliberately absent, so generic entity APIs can never create or read them.
pub const CLASSES: &[ClassDef] = &[
    class("self_profile", "自我本体", "agent", &["self"], &["self"], "owner"),
    class("person", "人物", "agent", &["physical"], &["social"], "owner"),
    class("supplier", "供应商", "agent", &["physical"], &["finance"], "owner"),
    class("identity", "自我认同", "relation", &["self", "mental"], &["self"], "owner"),
    class("self_relation", "自我关系", "relation", &["self"], &["self"], "owner"),
    class("relationship", "人际关系", "relation", &["self"], &["social"], "owner"),
    class("body_metric", "生理自我指标", "observation", &["self", "physical"], &["health"], "owner"),
    class("psychological_metric", "心理自我认定", "observation", &["self", "mental"], &["self"], "owner"),
    class("interaction", "互动记录", "event", &["self"], &["social"], "owner"),
    class("directive", "执行方向", "resource", &["digital"], &[], "owner"),
    class("channel", "会话", "resource", &["digital"], &[], "owner"),
    class("message", "消息", "event", &["digital"], &[], "owner"),
    class("note", "知识条目", "resource", &["digital"], &["learning"], "owner"),
    class("proposal", "建议", "commitment", &["digital"], &[], "agent-propose"),
    class("task", "任务 / 工单", "commitment", &["digital", "physical"], &[], "owner-confirm"),
    class("resource", "资源 / 物料", "resource", &["physical"], &["life"], "owner-confirm"),
    class("location", "库位", "resource", &["physical"], &["life"], "owner"),
    class("stock_movement", "库存流水", "event", &["physical"], &["life"], "owner-confirm"),
    class("cash_entry", "现金流水", "event", &["physical"], &["finance"], "owner-confirm"),
    class("purchase_order", "采购订单", "commitment", &["physical"], &["finance"], "owner-confirm"),
    class("purchase_order_line", "采购订单行", "commitment", &["physical"], &["finance"], "owner-confirm"),
    class("agent_import", "Agent 导入", "event", &["digital"], &[], "agent-propose"),
    class("agent_run", "路由运行", "event", &["digital"], &[], "system"),
    class("model_endpoint", "本地模型", "resource", &["digital"], &[], "owner"),
    class("route_rule", "路由规则", "resource", &["digital"], &[], "owner"),
];
/// Encrypted mental vault classes (shown in the schema, never stored as entities).
pub const VAULT_CLASSES: &[(&str, &str)] = &[("principle", "原则"), ("goal", "目标"), ("reflection", "反思")];

const REFERABLE: &[&str] = &[
    "note",
    "task",
    "directive",
    "resource",
    "person",
    "supplier",
    "location",
    "relationship",
    "identity",
    "self_relation",
    "self_profile",
    "body_metric",
    "psychological_metric",
];
const SELF_ASPECTS: &[&str] = &["body_metric", "psychological_metric", "self_relation", "identity"];

pub const PREDICATES: &[PredicateDef] = &[
    PredicateDef {
        id: "implements",
        label: "落实",
        layer: "digital",
        owner_settable: true,
        agent_proposable: true,
        signatures: &[sig(&["task"], &["directive"])],
    },
    PredicateDef {
        id: "uses",
        label: "使用",
        layer: "erp",
        owner_settable: true,
        agent_proposable: true,
        signatures: &[sig(&["task"], &["resource"])],
    },
    PredicateDef {
        id: "references",
        label: "关联",
        layer: "digital",
        owner_settable: true,
        agent_proposable: true,
        signatures: &[sig(REFERABLE, REFERABLE)],
    },
    PredicateDef {
        id: "derived_from",
        label: "源于",
        layer: "provenance",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[
            sig(&["proposal"], &["message", "resource", "relationship", "agent_import"]),
            sig(
                &["task", "resource", "note", "person", "supplier", "location"],
                &["proposal"],
            ),
        ],
    },
    PredicateDef {
        id: "belongs_to",
        label: "属于",
        layer: "digital",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["message"], &["channel"])],
    },
    PredicateDef {
        id: "records",
        label: "记录",
        layer: "erp",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["stock_movement"], &["resource"])],
    },
    PredicateDef {
        id: "describes_self",
        label: "描述自我",
        layer: "self",
        owner_settable: true,
        agent_proposable: false,
        signatures: &[sig(SELF_ASPECTS, &["self_profile"])],
    },
    PredicateDef {
        id: "relates_to_self",
        label: "与自我关系",
        layer: "self",
        owner_settable: true,
        agent_proposable: false,
        signatures: &[sig(
            &["self_relation", "body_metric", "psychological_metric", "identity", "relationship"],
            &["self_profile"],
        )],
    },
    PredicateDef {
        id: "measures",
        label: "测量",
        layer: "self",
        owner_settable: true,
        agent_proposable: false,
        signatures: &[sig(
            &["body_metric", "psychological_metric"],
            &["self_profile", "resource", "task", "relationship"],
        )],
    },
    PredicateDef {
        id: "supports",
        label: "支持",
        layer: "self",
        owner_settable: true,
        agent_proposable: false,
        signatures: &[sig(
            &["resource", "task", "note", "directive", "relationship", "person"],
            &["self_profile", "body_metric", "psychological_metric", "identity"],
        )],
    },
    PredicateDef {
        id: "influences",
        label: "影响",
        layer: "self",
        owner_settable: true,
        agent_proposable: false,
        signatures: &[sig(
            &[
                "self_profile",
                "body_metric",
                "psychological_metric",
                "identity",
                "self_relation",
                "relationship",
            ],
            &["task", "resource", "directive"],
        )],
    },
    PredicateDef {
        id: "involves",
        label: "涉及",
        layer: "external",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["relationship"], &["person"])],
    },
    PredicateDef {
        id: "occurs_in",
        label: "发生于",
        layer: "external",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["interaction"], &["relationship"])],
    },
    PredicateDef {
        id: "supplies",
        label: "供应",
        layer: "erp",
        owner_settable: true,
        agent_proposable: true,
        signatures: &[sig(&["supplier"], &["resource", "purchase_order"])],
    },
    PredicateDef {
        id: "contains",
        label: "包含",
        layer: "erp",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["purchase_order"], &["purchase_order_line"])],
    },
    PredicateDef {
        id: "orders",
        label: "订购",
        layer: "erp",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["purchase_order_line"], &["resource"])],
    },
    PredicateDef {
        id: "receives",
        label: "收货",
        layer: "erp",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["stock_movement"], &["purchase_order_line"])],
    },
    PredicateDef {
        id: "pays",
        label: "付款",
        layer: "erp",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["cash_entry"], &["purchase_order"])],
    },
    PredicateDef {
        id: "stored_at",
        label: "存放于",
        layer: "erp",
        owner_settable: true,
        agent_proposable: true,
        signatures: &[sig(&["resource"], &["location"])],
    },
    PredicateDef {
        id: "located_in",
        label: "位于",
        layer: "erp",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["location"], &["location"])],
    },
    PredicateDef {
        id: "processes",
        label: "处理",
        layer: "provenance",
        owner_settable: false,
        agent_proposable: false,
        signatures: &[sig(&["agent_run"], &["agent_import"])],
    },
];

pub fn class_def(kind: &str) -> Option<&'static ClassDef> {
    CLASSES.iter().find(|c| c.id == kind)
}
pub fn predicate_def(id: &str) -> Option<&'static PredicateDef> {
    PREDICATES.iter().find(|p| p.id == id)
}
pub fn label(kind: &str) -> &'static str {
    class_def(kind).map(|c| c.label).unwrap_or("实体")
}
pub fn worlds(kind: &str) -> Vec<String> {
    class_def(kind)
        .map(|c| c.worlds.iter().map(|w| w.to_string()).collect())
        .unwrap_or_else(|| vec!["digital".into()])
}
pub fn relation_valid(subject: &str, predicate: &str, object: &str) -> bool {
    predicate_def(predicate).is_some_and(|p| {
        p.signatures
            .iter()
            .any(|s| s.domain.contains(&subject) && s.range.contains(&object))
    })
}
pub fn validate_text(text: &str, max: usize) -> Result<()> {
    require(
        !text.trim().is_empty() && text.chars().count() <= max,
        "文本为空或超过长度限制",
    )
}
/// Optional text: may be empty, but never longer than `max` characters.
pub fn validate_optional(text: &str, max: usize, message: &str) -> Result<()> {
    require(text.chars().count() <= max, message)
}
pub fn validate_values(values: &[String]) -> Result<()> {
    require(
        values.len() <= 3
            && values
                .iter()
                .all(|v| VALUE_AXES.iter().any(|(id, _)| id == v)),
        "评价维度不合法",
    )
}
pub fn validate_scenes(scenes: &[String]) -> Result<()> {
    require(
        scenes.len() <= SCENES.len()
            && scenes.iter().all(|s| SCENES.iter().any(|(id, ..)| id == s))
            && scenes
                .iter()
                .enumerate()
                .all(|(i, s)| !scenes[..i].contains(s)),
        "人生场景不合法",
    )
}
/// Validated owner-chosen scenes, or the class defaults when none are chosen.
pub fn scenes_for(kind: &str, requested: &[String]) -> Result<Vec<String>> {
    validate_scenes(requested)?;
    if !requested.is_empty() {
        return Ok(requested.to_vec());
    }
    Ok(class_def(kind)
        .map(|c| c.scenes.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default())
}
pub fn is_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}
pub fn schema() -> Value {
    let pairs = |items: &[(&str, &str)]| -> Vec<Value> {
        items
            .iter()
            .map(|(id, label)| json!({"id": id, "label": label}))
            .collect()
    };
    let triples = |items: &[(&str, &str, &str)]| -> Vec<Value> {
        items
            .iter()
            .map(|(id, label, description)| json!({"id": id, "label": label, "description": description}))
            .collect()
    };
    let mut classes: Vec<Value> = VAULT_CLASSES
        .iter()
        .map(|(id, label)| {
            json!({"id": id, "label": label, "category": "resource", "world": "mental", "worlds": ["mental"], "authority": "owner", "storage": "vault", "default_scenes": ["self"]})
        })
        .collect();
    classes.extend(CLASSES.iter().map(|c| {
        json!({"id": c.id, "label": c.label, "category": c.category, "world": c.worlds[0], "worlds": c.worlds, "authority": c.authority, "storage": "operations", "default_scenes": c.scenes})
    }));
    let predicates: Vec<Value> = PREDICATES
        .iter()
        .map(|p| {
            let mut domain: Vec<&str> = Vec::new();
            let mut range: Vec<&str> = Vec::new();
            for s in p.signatures {
                for kind in s.domain {
                    if !domain.contains(kind) {
                        domain.push(kind);
                    }
                }
                for kind in s.range {
                    if !range.contains(kind) {
                        range.push(kind);
                    }
                }
            }
            json!({
                "id": p.id, "label": p.label, "layer": p.layer,
                "owner_settable": p.owner_settable, "agent_proposable": p.agent_proposable,
                "domain": domain, "range": range,
                "signatures": p.signatures.iter().map(|s| json!({"domain": s.domain, "range": s.range})).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "version": 2,
        "worlds": pairs(WORLDS),
        "value_axes": pairs(VALUE_AXES),
        "scenes": triples(SCENES),
        "categories": triples(CATEGORIES),
        "layers": triples(LAYERS),
        "classes": classes,
        "predicates": predicates,
    })
}
/// Idempotent SQL that aligns the database constraints with the registry.
/// Every identifier is a static lowercase name (checked by the tests below).
pub fn sync_sql() -> String {
    let quote = |id: &str| format!("'{id}'");
    let classes = CLASSES
        .iter()
        .map(|c| format!("({})", quote(c.id)))
        .collect::<Vec<_>>()
        .join(",");
    let predicates = PREDICATES
        .iter()
        .map(|p| quote(p.id))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "INSERT INTO ontology_classes(id) VALUES {classes} ON CONFLICT DO NOTHING; \
         ALTER TABLE relations DROP CONSTRAINT IF EXISTS relations_predicate_check; \
         ALTER TABLE relations ADD CONSTRAINT relations_predicate_check CHECK(predicate IN ({predicates}));"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(value: &str) -> bool {
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
    }

    #[test]
    fn registry_identifiers_are_unique_and_safe_for_sql() {
        for (i, c) in CLASSES.iter().enumerate() {
            assert!(identifier(c.id), "{}", c.id);
            assert!(CLASSES[..i].iter().all(|o| o.id != c.id), "{}", c.id);
            assert!(!c.worlds.is_empty() && c.worlds.iter().all(|w| WORLDS.iter().any(|(id, _)| id == w)));
            assert!(c.scenes.iter().all(|s| SCENES.iter().any(|(id, ..)| id == s)));
            assert!(CATEGORIES.iter().any(|(id, ..)| *id == c.category));
        }
        for (i, p) in PREDICATES.iter().enumerate() {
            assert!(identifier(p.id), "{}", p.id);
            assert!(PREDICATES[..i].iter().all(|o| o.id != p.id), "{}", p.id);
            assert!(LAYERS.iter().any(|(id, ..)| *id == p.layer));
            assert!(!p.agent_proposable || p.owner_settable, "{}", p.id);
        }
    }

    #[test]
    fn every_signature_refers_to_registered_classes() {
        for p in PREDICATES {
            for s in p.signatures {
                for kind in s.domain.iter().chain(s.range) {
                    assert!(class_def(kind).is_some(), "{} uses {kind}", p.id);
                }
            }
        }
    }

    #[test]
    fn relation_rules_keep_system_provenance_out_of_owner_hands() {
        assert!(relation_valid("task", "implements", "directive"));
        assert!(!relation_valid("resource", "implements", "resource"));
        assert!(relation_valid("proposal", "derived_from", "message"));
        assert!(relation_valid("task", "derived_from", "proposal"));
        assert!(!relation_valid("task", "derived_from", "message"));
        assert!(relation_valid("stock_movement", "records", "resource"));
        assert!(relation_valid("relationship", "involves", "person"));
        assert!(!relation_valid("task", "unknown", "resource"));
        for id in ["derived_from", "belongs_to", "records", "receives", "pays", "processes"] {
            assert!(!predicate_def(id).unwrap().owner_settable, "{id}");
        }
    }

    #[test]
    fn scenes_are_validated_and_defaulted() {
        assert_eq!(scenes_for("body_metric", &[]).unwrap(), vec!["health"]);
        assert_eq!(scenes_for("task", &["work".into()]).unwrap(), vec!["work"]);
        assert!(scenes_for("task", &["unknown".into()]).is_err());
        assert!(scenes_for("task", &["work".into(), "work".into()]).is_err());
        assert!(sync_sql().contains("'relationship'"));
    }
}
