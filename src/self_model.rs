//! Self ontology: the owner's relation to themself. Physiological observations,
//! mainstream psychological self-concept measures (catalog metadata only — no
//! copyrighted item text), identity statements and stances toward the self.
//! Values are owner observations: never a diagnosis and never merged into one score.
use crate::{
    agents,
    error::{require, Error, Result},
    ontology::{self, validate_optional, validate_text, validate_values, Entity},
    relationships::{INTERACTION_CHANNELS, RELATIONSHIP_TYPES},
    store::{id, Store},
    Actor, AppState,
};
use axum::{extract::State, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};

pub struct MetricDef {
    pub id: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    /// "better", "worse" or "neutral" when the value increases.
    pub higher: &'static str,
}
pub struct InstrumentDef {
    pub id: &'static str,
    pub abbr: &'static str,
    pub name: &'static str,
    /// "body", "psychology" or "relationship".
    pub domain: &'static str,
    pub family: &'static str,
    pub construct: &'static str,
    pub items: u16,
    pub response: &'static str,
    pub citation: &'static str,
    pub scenes: &'static [&'static str],
    pub metrics: &'static [MetricDef],
}
const fn m(
    id: &'static str,
    label: &'static str,
    unit: &'static str,
    min: f64,
    max: f64,
    higher: &'static str,
) -> MetricDef {
    MetricDef {
        id,
        label,
        unit,
        min,
        max,
        higher,
    }
}

pub const INSTRUMENTS: &[InstrumentDef] = &[
    InstrumentDef {
        id: "sleep",
        abbr: "睡眠",
        name: "睡眠观察",
        domain: "body",
        family: "生理自我",
        construct: "睡眠时长与主观睡眠质量",
        items: 0,
        response: "本人记录或设备读数",
        citation: "",
        scenes: &["health"],
        metrics: &[
            m("sleep.duration", "睡眠时长", "小时", 0.0, 24.0, "neutral"),
            m("sleep.quality", "主观睡眠质量", "1–5", 1.0, 5.0, "better"),
        ],
    },
    InstrumentDef {
        id: "cardio",
        abbr: "心血管",
        name: "心血管读数",
        domain: "body",
        family: "生理自我",
        construct: "静息心率、心率变异性与血压",
        items: 0,
        response: "设备读数",
        citation: "",
        scenes: &["health"],
        metrics: &[
            m("cardio.resting_hr", "静息心率", "次/分", 20.0, 250.0, "neutral"),
            m("cardio.hrv_rmssd", "心率变异性 RMSSD", "毫秒", 0.0, 500.0, "neutral"),
            m("cardio.bp_systolic", "收缩压", "mmHg", 50.0, 260.0, "neutral"),
            m("cardio.bp_diastolic", "舒张压", "mmHg", 30.0, 180.0, "neutral"),
        ],
    },
    InstrumentDef {
        id: "body",
        abbr: "体成分",
        name: "体成分测量",
        domain: "body",
        family: "生理自我",
        construct: "体重、体脂率与腰围",
        items: 0,
        response: "本人测量或设备读数",
        citation: "",
        scenes: &["health"],
        metrics: &[
            m("body.weight", "体重", "千克", 1.0, 500.0, "neutral"),
            m("body.fat_percent", "体脂率", "%", 1.0, 80.0, "neutral"),
            m("body.waist", "腰围", "厘米", 20.0, 250.0, "neutral"),
        ],
    },
    InstrumentDef {
        id: "activity",
        abbr: "活动",
        name: "身体活动",
        domain: "body",
        family: "生理自我",
        construct: "日常步数与中高强度活动时间",
        items: 0,
        response: "设备读数或本人记录",
        citation: "",
        scenes: &["health"],
        metrics: &[
            m("activity.steps", "步数", "步", 0.0, 200000.0, "neutral"),
            m("activity.active_minutes", "中高强度活动", "分钟", 0.0, 1440.0, "neutral"),
        ],
    },
    InstrumentDef {
        id: "subjective",
        abbr: "体感",
        name: "主观体感（数字评定量表 NRS）",
        domain: "body",
        family: "生理自我",
        construct: "精力与疼痛的主观强度",
        items: 0,
        response: "0–10 数字评定",
        citation: "",
        scenes: &["health"],
        metrics: &[
            m("subjective.energy", "精力", "0–10", 0.0, 10.0, "better"),
            m("subjective.pain", "疼痛", "0–10", 0.0, 10.0, "worse"),
        ],
    },
    InstrumentDef {
        id: "rses",
        abbr: "RSES",
        name: "Rosenberg 自尊量表",
        domain: "psychology",
        family: "自我评价",
        construct: "整体自尊：对自身价值的总体评价",
        items: 10,
        response: "10 题，4 点计分（含反向题），总分 10–40",
        citation: "Rosenberg, M. (1965). Society and the adolescent self-image. Princeton University Press.",
        scenes: &["self"],
        metrics: &[m("rses.total", "自尊总分", "分", 10.0, 40.0, "better")],
    },
    InstrumentDef {
        id: "sccs",
        abbr: "SCCS",
        name: "自我概念清晰度量表",
        domain: "psychology",
        family: "自我评价",
        construct: "自我概念清晰度：对“我是谁”的认识是否清楚、一致、稳定",
        items: 12,
        response: "12 题，5 点计分（含反向题），总分 12–60",
        citation: "Campbell, J. D., Trapnell, P. D., Heine, S. J., Katz, I. M., Lavallee, L. F., & Lehman, D. R. (1996). Journal of Personality and Social Psychology, 70(1), 141–156.",
        scenes: &["self"],
        metrics: &[m("sccs.total", "自我概念清晰度", "分", 12.0, 60.0, "better")],
    },
    InstrumentDef {
        id: "cses",
        abbr: "CSES",
        name: "核心自我评价量表",
        domain: "psychology",
        family: "自我评价",
        construct: "核心自我评价：自尊、一般自我效能、控制点与情绪稳定性的共同核心",
        items: 12,
        response: "12 题，5 点计分（含反向题），取均分 1–5",
        citation: "Judge, T. A., Erez, A., Bono, J. E., & Thoresen, C. J. (2003). Personnel Psychology, 56(2), 303–331.",
        scenes: &["self", "work"],
        metrics: &[m("cses.mean", "核心自我评价", "均分", 1.0, 5.0, "better")],
    },
    InstrumentDef {
        id: "gses",
        abbr: "GSES",
        name: "一般自我效能感量表",
        domain: "psychology",
        family: "自我评价",
        construct: "一般自我效能：相信自己能应对各种挑战",
        items: 10,
        response: "10 题，4 点计分，总分 10–40",
        citation: "Schwarzer, R., & Jerusalem, M. (1995). Generalized Self-Efficacy scale. In J. Weinman, S. Wright, & M. Johnston (Eds.), Measures in health psychology (pp. 35–37). NFER-NELSON.",
        scenes: &["self", "learning"],
        metrics: &[m("gses.total", "一般自我效能", "分", 10.0, 40.0, "better")],
    },
    InstrumentDef {
        id: "scs_sf",
        abbr: "SCS-SF",
        name: "自悯量表简版",
        domain: "psychology",
        family: "自我评价",
        construct: "自我关怀：困难时以善意、共通人性与正念对待自己",
        items: 12,
        response: "12 题，5 点计分（消极条目反向），取均分 1–5",
        citation: "Raes, F., Pommier, E., Neff, K. D., & Van Gucht, D. (2011). Clinical Psychology & Psychotherapy, 18(3), 250–255.",
        scenes: &["self"],
        metrics: &[m("scs_sf.mean", "自我关怀", "均分", 1.0, 5.0, "better")],
    },
    InstrumentDef {
        id: "bpnsfs",
        abbr: "BPNSFS",
        name: "基本心理需要满足与受挫量表",
        domain: "psychology",
        family: "动机与需要",
        construct: "自我决定理论的三种基本心理需要：自主、胜任、关系的满足与受挫",
        items: 24,
        response: "24 题，5 点计分，6 个分量表各取均分 1–5",
        citation: "Chen, B., Vansteenkiste, M., Beyers, W., Boone, L., Deci, E. L., Van der Kaap-Deeder, J., et al. (2015). Motivation and Emotion, 39(2), 216–236.",
        scenes: &["self", "work", "social"],
        metrics: &[
            m("bpnsfs.autonomy_satisfaction", "自主满足", "均分", 1.0, 5.0, "better"),
            m("bpnsfs.autonomy_frustration", "自主受挫", "均分", 1.0, 5.0, "worse"),
            m("bpnsfs.competence_satisfaction", "胜任满足", "均分", 1.0, 5.0, "better"),
            m("bpnsfs.competence_frustration", "胜任受挫", "均分", 1.0, 5.0, "worse"),
            m("bpnsfs.relatedness_satisfaction", "关系满足", "均分", 1.0, 5.0, "better"),
            m("bpnsfs.relatedness_frustration", "关系受挫", "均分", 1.0, 5.0, "worse"),
        ],
    },
    InstrumentDef {
        id: "swls",
        abbr: "SWLS",
        name: "生活满意度量表",
        domain: "psychology",
        family: "幸福感",
        construct: "对自己生活整体的认知评价",
        items: 5,
        response: "5 题，7 点计分，总分 5–35",
        citation: "Diener, E., Emmons, R. A., Larsen, R. J., & Griffin, S. (1985). Journal of Personality Assessment, 49(1), 71–75.",
        scenes: &["self", "life"],
        metrics: &[m("swls.total", "生活满意度", "分", 5.0, 35.0, "better")],
    },
    InstrumentDef {
        id: "who5",
        abbr: "WHO-5",
        name: "WHO-5 幸福感指数",
        domain: "psychology",
        family: "幸福感",
        construct: "近两周的主观幸福感",
        items: 5,
        response: "5 题，0–5 计分；原始分 0–25，×4 为百分制 0–100",
        citation: "Topp, C. W., Østergaard, S. D., Søndergaard, S., & Bech, P. (2015). Psychotherapy and Psychosomatics, 84(3), 167–176.",
        scenes: &["self", "health"],
        metrics: &[m("who5.percent", "幸福感指数", "0–100", 0.0, 100.0, "better")],
    },
    InstrumentDef {
        id: "pwb",
        abbr: "PWB",
        name: "Ryff 心理幸福感量表",
        domain: "psychology",
        family: "幸福感",
        construct: "心理幸福感的六个维度",
        items: 0,
        response: "版本不同（18/42/84 题），6 或 7 点计分，各维度取均分",
        citation: "Ryff, C. D. (1989). Journal of Personality and Social Psychology, 57(6), 1069–1081.",
        scenes: &["self"],
        metrics: &[
            m("pwb.autonomy", "自主", "均分", 1.0, 7.0, "better"),
            m("pwb.environmental_mastery", "环境掌控", "均分", 1.0, 7.0, "better"),
            m("pwb.personal_growth", "个人成长", "均分", 1.0, 7.0, "better"),
            m("pwb.positive_relations", "积极关系", "均分", 1.0, 7.0, "better"),
            m("pwb.purpose_in_life", "生活目的", "均分", 1.0, 7.0, "better"),
            m("pwb.self_acceptance", "自我接纳", "均分", 1.0, 7.0, "better"),
        ],
    },
    InstrumentDef {
        id: "ipanas_sf",
        abbr: "I-PANAS-SF",
        name: "国际积极与消极情感量表简版",
        domain: "psychology",
        family: "幸福感",
        construct: "积极情感与消极情感（两个独立维度）",
        items: 10,
        response: "10 题，5 点计分；积极、消极情感各 5–25",
        citation: "Thompson, E. R. (2007). Journal of Cross-Cultural Psychology, 38(2), 227–242.",
        scenes: &["self", "emotion"],
        metrics: &[
            m("ipanas_sf.positive", "积极情感", "分", 5.0, 25.0, "better"),
            m("ipanas_sf.negative", "消极情感", "分", 5.0, 25.0, "worse"),
        ],
    },
    InstrumentDef {
        id: "pss10",
        abbr: "PSS-10",
        name: "知觉压力量表",
        domain: "psychology",
        family: "压力与韧性",
        construct: "近一个月生活情境被感知为不可预测、不可控与超负荷的程度",
        items: 10,
        response: "10 题，0–4 计分（含反向题），总分 0–40，越高压力越大",
        citation: "Cohen, S., Kamarck, T., & Mermelstein, R. (1983). Journal of Health and Social Behavior, 24(4), 385–396.",
        scenes: &["self", "health", "work"],
        metrics: &[m("pss10.total", "知觉压力", "分", 0.0, 40.0, "worse")],
    },
    InstrumentDef {
        id: "brs",
        abbr: "BRS",
        name: "简易心理韧性量表",
        domain: "psychology",
        family: "压力与韧性",
        construct: "从压力中恢复（反弹）的能力",
        items: 6,
        response: "6 题，5 点计分（含反向题），取均分 1–5",
        citation: "Smith, B. W., Dalen, J., Wiggins, K., Tooley, E., Christopher, P., & Bernard, J. (2008). International Journal of Behavioral Medicine, 15(3), 194–200.",
        scenes: &["self", "health"],
        metrics: &[m("brs.mean", "心理韧性", "均分", 1.0, 5.0, "better")],
    },
    InstrumentDef {
        id: "bfi2s",
        abbr: "BFI-2-S",
        name: "大五人格量表第二版简版",
        domain: "psychology",
        family: "人格与觉察",
        construct: "五大人格领域（特质无好坏之分）",
        items: 30,
        response: "30 题，5 点计分，5 个领域各取均分 1–5",
        citation: "Soto, C. J., & John, O. P. (2017). Journal of Research in Personality, 68, 69–81.",
        scenes: &["self"],
        metrics: &[
            m("bfi2s.extraversion", "外向性", "均分", 1.0, 5.0, "neutral"),
            m("bfi2s.agreeableness", "宜人性", "均分", 1.0, 5.0, "neutral"),
            m("bfi2s.conscientiousness", "尽责性", "均分", 1.0, 5.0, "neutral"),
            m("bfi2s.negative_emotionality", "负性情绪", "均分", 1.0, 5.0, "neutral"),
            m("bfi2s.open_mindedness", "开放性", "均分", 1.0, 5.0, "neutral"),
        ],
    },
    InstrumentDef {
        id: "maas",
        abbr: "MAAS",
        name: "正念注意觉知量表",
        domain: "psychology",
        family: "人格与觉察",
        construct: "日常生活中对当下的注意与觉知",
        items: 15,
        response: "15 题，6 点计分，取均分 1–6",
        citation: "Brown, K. W., & Ryan, R. M. (2003). Journal of Personality and Social Psychology, 84(4), 822–848.",
        scenes: &["self", "health"],
        metrics: &[m("maas.mean", "正念觉知", "均分", 1.0, 6.0, "better")],
    },
    InstrumentDef {
        id: "wleis",
        abbr: "WLEIS",
        name: "Wong 与 Law 情绪智力量表",
        domain: "psychology",
        family: "人格与觉察",
        construct: "情绪智力的四个维度",
        items: 16,
        response: "16 题，7 点计分，4 个维度各取均分 1–7",
        citation: "Wong, C.-S., & Law, K. S. (2002). The Leadership Quarterly, 13(3), 243–274.",
        scenes: &["self", "emotion", "work"],
        metrics: &[
            m("wleis.self_emotion_appraisal", "自我情绪评估", "均分", 1.0, 7.0, "better"),
            m("wleis.others_emotion_appraisal", "他人情绪评估", "均分", 1.0, 7.0, "better"),
            m("wleis.use_of_emotion", "情绪运用", "均分", 1.0, 7.0, "better"),
            m("wleis.regulation_of_emotion", "情绪调节", "均分", 1.0, 7.0, "better"),
        ],
    },
    InstrumentDef {
        id: "ecr_r",
        abbr: "ECR-R",
        name: "亲密关系经历量表修订版",
        domain: "psychology",
        family: "亲密关系",
        construct: "成人依恋的两个维度：依恋焦虑与依恋回避",
        items: 36,
        response: "36 题，7 点计分，两个维度各取均分 1–7",
        citation: "Fraley, R. C., Waller, N. G., & Brennan, K. A. (2000). Journal of Personality and Social Psychology, 78(2), 350–365.",
        scenes: &["emotion"],
        metrics: &[
            m("ecr_r.anxiety", "依恋焦虑", "均分", 1.0, 7.0, "worse"),
            m("ecr_r.avoidance", "依恋回避", "均分", 1.0, 7.0, "worse"),
        ],
    },
    InstrumentDef {
        id: "ios",
        abbr: "IOS",
        name: "他人融入自我量表",
        domain: "relationship",
        family: "关系测量",
        construct: "关系亲密度：他人在多大程度上融入自我",
        items: 1,
        response: "单题图示，1（完全分离）– 7（高度重叠）",
        citation: "Aron, A., Aron, E. N., & Smollan, D. (1992). Journal of Personality and Social Psychology, 63(4), 596–612.",
        scenes: &["emotion", "social"],
        metrics: &[m("ios.closeness", "亲密度", "1–7", 1.0, 7.0, "neutral")],
    },
    InstrumentDef {
        id: "ras",
        abbr: "RAS",
        name: "关系评估量表",
        domain: "relationship",
        family: "关系测量",
        construct: "对一段关系的总体满意度",
        items: 7,
        response: "7 题，5 点计分（含反向题），取均分 1–5",
        citation: "Hendrick, S. S. (1988). Journal of Marriage and the Family, 50(1), 93–98.",
        scenes: &["emotion", "social"],
        metrics: &[m("ras.mean", "关系满意度", "均分", 1.0, 5.0, "better")],
    },
];
/// Stances toward the self (relations with oneself).
pub const STANCES: &[(&str, &str, &str)] = &[
    (
        "acceptance",
        "自我接纳",
        "接受现在的自己，包括优点与不足（Ryff）",
    ),
    ("compassion", "自我关怀", "在困难或失败时善待自己（Neff）"),
    ("criticism", "自我苛责", "对自己的严厉评判与要求"),
    ("trust", "自我信任", "相信自己的判断与能力"),
    (
        "ideal",
        "理想自我",
        "我希望成为的样子（Higgins 自我差异理论）",
    ),
    (
        "ought",
        "应然自我",
        "我认为应该成为的样子（Higgins 自我差异理论）",
    ),
    ("boundary", "自我边界", "我为自己设定并维护的界限"),
    ("growth", "成长意图", "我正在有意培养的方面"),
];
/// Twenty Statements Test ("Who am I?") coding categories (Kuhn & McPartland, 1954).
pub const IDENTITY_CATEGORIES: &[(&str, &str, &str)] = &[
    ("physical", "身体特征", "身体、外貌与生理方面的自我描述"),
    ("social", "社会角色", "职业、家庭、群体等社会身份"),
    ("reflective", "心理特质", "性格、价值、能力与偏好"),
    ("oceanic", "整体存在", "超越具体角色的整体性自我感"),
];
const SELF_TARGETS: &[&str] = &[
    "task",
    "resource",
    "directive",
    "note",
    "body_metric",
    "psychological_metric",
    "identity",
    "relationship",
    "person",
];
const VISIBILITY_KINDS: &[&str] = &[
    "self_profile",
    "body_metric",
    "psychological_metric",
    "identity",
    "self_relation",
    "relationship",
    "person",
];

pub fn instrument(id: &str) -> Option<&'static InstrumentDef> {
    INSTRUMENTS.iter().find(|i| i.id == id)
}
pub fn ensure_self_profile(store: &Store) -> Result<Entity> {
    if let Some(profile) = store.entities_of(&["self_profile"])?.into_iter().next() {
        return Ok(profile);
    }
    store.insert(
        "self_profile",
        "我的自我本体",
        &["existence".into()],
        json!({"schema":"prm-self-v2","agent_visible":false,"description":"由本人维护的生理、心理、身份与关系的自我观察锚点"}),
        "active",
    )
}
fn pairs(items: &[(&str, &str, &str)]) -> Vec<Value> {
    items
        .iter()
        .map(|(id, label, description)| json!({"id": id, "label": label, "description": description}))
        .collect()
}
pub async fn catalog(Extension(actor): Extension<Actor>) -> Result<Json<Value>> {
    actor.owner()?;
    let instruments: Vec<Value> = INSTRUMENTS
        .iter()
        .map(|i| {
            json!({
                "id": i.id, "abbr": i.abbr, "name": i.name, "domain": i.domain, "family": i.family,
                "construct": i.construct, "items": i.items, "response": i.response,
                "citation": i.citation, "scenes": i.scenes,
                "metrics": i.metrics.iter().map(|m| json!({"id": m.id, "label": m.label, "unit": m.unit, "min": m.min, "max": m.max, "higher": m.higher})).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Json(json!({
        "instruments": instruments,
        "stances": pairs(STANCES),
        "identity_categories": pairs(IDENTITY_CATEGORIES),
        "relationship_types": RELATIONSHIP_TYPES.iter().map(|(id, label, scene)| json!({"id": id, "label": label, "scene": scene})).collect::<Vec<_>>(),
        "interaction_channels": INTERACTION_CHANNELS.iter().map(|(id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
        "notice": "量表目录只提供名称、出处与计分范围，不包含题目。请使用经授权的版本作答后录入得分；结果是本人观察，不是诊断，也不会合成为单一分数。",
    })))
}
pub async fn profile(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    let profile = ensure_self_profile(&inner.store)?;
    let entities = inner.store.entities_of(&[
        "body_metric",
        "psychological_metric",
        "identity",
        "self_relation",
    ])?;
    let of = |kinds: &[&str]| {
        entities
            .iter()
            .filter(|e| kinds.contains(&e.kind.as_str()))
            .collect::<Vec<_>>()
    };
    Ok(Json(json!({
        "profile": profile,
        "indicators": of(&["body_metric", "psychological_metric"]),
        "identities": of(&["identity"]),
        "self_relations": of(&["self_relation"]),
    })))
}
fn observed_at_valid(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok() || ontology::is_date(value)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub metric: String,
    pub value: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssessmentInput {
    pub instrument: String,
    pub observed_at: String,
    pub scores: Vec<Score>,
    #[serde(default)]
    pub custom_label: String,
    #[serde(default)]
    pub custom_unit: String,
    #[serde(default)]
    pub custom_domain: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub agent_visible: bool,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>,
}
/// Record one assessment: one observation entity per (sub)scale score.
pub async fn assessment(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<AssessmentInput>,
) -> Result<Json<Value>> {
    actor.owner()?;
    require(
        observed_at_valid(&body.observed_at),
        "观察时间必须是 RFC3339 或 YYYY-MM-DD",
    )?;
    validate_optional(&body.notes, 4000, "备注过长")?;
    validate_values(&body.values)?;
    ontology::validate_scenes(&body.scenes)?;
    require(
        !body.scores.is_empty() && body.scores.len() <= 12,
        "请填写至少一项得分",
    )?;
    let mut readings: Vec<(&str, String, Value)> = Vec::new();
    let default_scenes: Vec<String> = if body.instrument == "custom" {
        validate_text(&body.custom_label, 40)?;
        validate_optional(&body.custom_unit, 20, "单位过长")?;
        let score = &body.scores[0];
        require(
            body.scores.len() == 1
                && score.metric == "custom"
                && score.value.is_finite()
                && score.value.abs() <= 1e9,
            "自定义指标只能记录一个有效数值",
        )?;
        let kind = match body.custom_domain.as_str() {
            "" | "body" => "body_metric",
            "psychology" => "psychological_metric",
            _ => return Err(Error::Bad("自定义指标类型不合法".into())),
        };
        readings.push((
            kind,
            body.custom_label.trim().to_string(),
            json!({"metric": "custom", "instrument": "custom", "label": body.custom_label.trim(), "value": score.value, "unit": body.custom_unit, "range": null, "higher": "neutral"}),
        ));
        ontology::scenes_for(kind, &[])?
    } else {
        let instrument = instrument(&body.instrument)
            .filter(|i| i.domain != "relationship")
            .ok_or(Error::Bad("量表或指标组不存在".into()))?;
        let kind = if instrument.domain == "body" {
            "body_metric"
        } else {
            "psychological_metric"
        };
        for (index, score) in body.scores.iter().enumerate() {
            let def = instrument
                .metrics
                .iter()
                .find(|m| m.id == score.metric)
                .ok_or(Error::Bad("指标不属于所选量表".into()))?;
            require(
                body.scores[..index]
                    .iter()
                    .all(|o| o.metric != score.metric),
                "同一指标只能填写一次",
            )?;
            require(
                score.value.is_finite() && score.value >= def.min && score.value <= def.max,
                &format!("{}应在 {}–{} 之间", def.label, def.min, def.max),
            )?;
            readings.push((
                kind,
                format!("{} · {}", instrument.abbr, def.label),
                json!({"metric": def.id, "instrument": instrument.id, "label": def.label, "value": score.value, "unit": def.unit, "range": [def.min, def.max], "higher": def.higher}),
            ));
        }
        instrument.scenes.iter().map(|s| s.to_string()).collect()
    };
    let scenes = if body.scenes.is_empty() {
        default_scenes
    } else {
        body.scenes.clone()
    };
    let inner = state.lock()?;
    let assessment_id = id();
    let recorded = inner.store.transaction(|store| {
        let profile = ensure_self_profile(store)?;
        for (kind, title, mut props) in readings {
            props["observed_at"] = json!(body.observed_at);
            props["notes"] = json!(body.notes);
            props["agent_visible"] = json!(body.agent_visible);
            props["assessment_id"] = json!(assessment_id);
            props["source"] = json!("self_report");
            let entity =
                store.insert_scoped(kind, &title, &body.values, &scenes, props, "observed")?;
            store.relate(&entity.id, "describes_self", &profile.id)?;
        }
        store.audit("owner", "self.assessment_recorded", &assessment_id)?;
        Ok(body.scores.len())
    })?;
    Ok(Json(
        json!({"assessment_id": assessment_id, "recorded": recorded}),
    ))
}
fn three() -> i64 {
    3
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityInput {
    pub statement: String,
    pub category: String,
    #[serde(default = "three")]
    pub centrality: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub agent_visible: bool,
}
/// A "我是……" statement in the Twenty Statements Test tradition.
pub async fn identity(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<IdentityInput>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.statement, 120)?;
    require(
        IDENTITY_CATEGORIES
            .iter()
            .any(|(id, ..)| *id == body.category),
        "自我认同类别不合法",
    )?;
    require((1..=5).contains(&body.centrality), "重要程度应为 1–5")?;
    validate_optional(&body.notes, 2000, "备注过长")?;
    validate_values(&body.values)?;
    let scenes = ontology::scenes_for("identity", &body.scenes)?;
    let inner = state.lock()?;
    let entity = inner.store.transaction(|store| {
        let profile = ensure_self_profile(store)?;
        let entity = store.insert_scoped(
            "identity",
            &body.statement,
            &body.values,
            &scenes,
            json!({"statement": body.statement.trim(), "category": body.category, "centrality": body.centrality, "notes": body.notes, "agent_visible": body.agent_visible}),
            "active",
        )?;
        store.relate(&entity.id, "describes_self", &profile.id)?;
        store.audit("owner", "self.identity_recorded", &entity.id)?;
        Ok(entity)
    })?;
    Ok(Json(json!(entity)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfRelationInput {
    pub title: String,
    pub stance: String,
    #[serde(default = "three")]
    pub intensity: i64,
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub scenes: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub agent_visible: bool,
}
/// A first-class relation of the self to itself, optionally pointing at an object
/// (for example “我需要规律休息” → a task or resource).
pub async fn self_relation(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<SelfRelationInput>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.title, 120)?;
    require(
        STANCES.iter().any(|(id, ..)| *id == body.stance),
        "自我关系立场不合法",
    )?;
    require((1..=5).contains(&body.intensity), "强度应为 1–5")?;
    validate_optional(&body.notes, 4000, "自我关系备注过长")?;
    validate_values(&body.values)?;
    let scenes = ontology::scenes_for("self_relation", &body.scenes)?;
    let inner = state.lock()?;
    let relation = inner.store.transaction(|store| {
        let profile = ensure_self_profile(store)?;
        if !body.target_id.is_empty() {
            let target = store.entity(&body.target_id)?;
            require(
                SELF_TARGETS.contains(&target.kind.as_str()),
                "自我关系目标类型不合法",
            )?;
        }
        let entity = store.insert_scoped(
            "self_relation",
            &body.title,
            &body.values,
            &scenes,
            json!({"stance": body.stance, "intensity": body.intensity, "target_id": body.target_id, "notes": body.notes, "agent_visible": body.agent_visible}),
            "active",
        )?;
        store.relate(&entity.id, "describes_self", &profile.id)?;
        if !body.target_id.is_empty() {
            store.relate(&entity.id, "references", &body.target_id)?;
        }
        store.audit("owner", "self.relation_recorded", &entity.id)?;
        Ok(entity)
    })?;
    Ok(Json(json!(relation)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Visibility {
    pub id: String,
    pub agent_visible: bool,
    pub expected_version: i64,
}
/// The owner decides, record by record, what the local model may see.
pub async fn visibility(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Visibility>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        let mut entity = store.entity(&body.id)?;
        require(
            VISIBILITY_KINDS.contains(&entity.kind.as_str()),
            "此记录不支持 Agent 可见性设置",
        )?;
        if entity.version != body.expected_version {
            return Err(Error::Conflict("记录已更新，请刷新后重试".into()));
        }
        entity.properties["agent_visible"] = json!(body.agent_visible);
        store.update(&entity)?;
        store.audit("owner", "self.visibility_changed", &body.id)
    })?;
    Ok(Json(json!({"ok": true})))
}
/// Exactly what a local model receives as the self-ontology core principles.
pub async fn agent_preview(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    Ok(Json(agents::router::principles_context(&inner.store, &[])?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_metrics_are_unique_and_ranged() {
        let mut seen: Vec<&str> = Vec::new();
        for instrument in INSTRUMENTS {
            assert!(!instrument.metrics.is_empty(), "{}", instrument.id);
            assert!(["body", "psychology", "relationship"].contains(&instrument.domain));
            assert!(instrument
                .scenes
                .iter()
                .all(|s| ontology::SCENES.iter().any(|(id, ..)| id == s)));
            for metric in instrument.metrics {
                assert!(metric.id.starts_with(instrument.id), "{}", metric.id);
                assert!(metric.min < metric.max, "{}", metric.id);
                assert!(["better", "worse", "neutral"].contains(&metric.higher));
                assert!(!seen.contains(&metric.id), "{}", metric.id);
                seen.push(metric.id);
            }
        }
        assert_eq!(instrument("rses").unwrap().metrics[0].max, 40.0);
    }
}
