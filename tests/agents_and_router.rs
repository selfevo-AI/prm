mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};

const IMPORT: &str = "/api/agent/import";

fn workplace_import() -> Value {
    json!({"scenes":["work"],"items":[
        {"ref":"r1","kind":"resource","title":"投影仪","attributes":{"unit":"台","location":"会议室"}},
        {"ref":"p1","kind":"person","title":"王经理","body":"项目对接人"},
        {"kind":"relation","subject":"p1","predicate":"references","object":"r1"},
        {"kind":"message","title":"周五前提交周报"}
    ]})
}
async fn proposals_of(h: &Harness, import_id: &Value) -> Vec<Value> {
    h.entities("proposal")
        .await
        .into_iter()
        .filter(|p| p["properties"]["source_id"] == *import_id)
        .collect()
}
async fn approve(h: &Harness, proposal: &Value, title: &str) -> (StatusCode, Value) {
    h.owner(
        &format!("/api/proposals/{}/decide", proposal["id"].as_str().unwrap()),
        Some(json!({"decision":"approve","expected_version":proposal["version"],"title":title})),
    )
    .await
}

#[tokio::test]
async fn external_agents_can_only_import_and_read_their_own_status() {
    let h = Harness::new().await;
    let (id, token) = h.connection("Claude Desktop").await;
    assert!(token.starts_with("prm_"));
    let list = h.owner("/api/agents/connections", None).await.1;
    assert!(!list.to_string().contains(&token));
    assert_eq!(list["executor_configured"], true);
    for path in [
        "/api/workspace",
        "/api/vault",
        "/api/export",
        "/api/agent/context",
        "/api/self/agent-preview",
        "/api/agents/connections",
    ] {
        assert_eq!(
            h.bearer(&token, path, None).await.0,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
    for path in [
        "/api/agent/jobs/claim",
        "/api/agent/proposals",
        "/api/entities",
        "/api/agents/models",
        "/api/proposals/x/decide",
    ] {
        assert_eq!(
            h.bearer(&token, path, Some(json!({}))).await.0,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
    assert_eq!(
        h.owner(IMPORT, Some(workplace_import())).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.agent(IMPORT, Some(workplace_import())).await.0,
        StatusCode::FORBIDDEN
    );
    let (status, outcome) = h.bearer(&token, IMPORT, Some(workplace_import())).await;
    assert_eq!(status, StatusCode::OK, "{outcome}");
    assert_eq!(outcome["status"], "processed");
    assert_eq!(outcome["proposals_created"], 4);
    for kind in ["resource", "person", "task"] {
        assert!(h.entities(kind).await.is_empty(), "{kind}");
    }
    let import_id = outcome["import_id"].as_str().unwrap().to_string();
    let (status, summary) = h
        .bearer(&token, &format!("/api/agent/imports/{import_id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["proposals"]["pending"], 4);
    assert!(!summary.to_string().contains("投影仪"));
    assert_eq!(
        h.bearer(&token, "/api/agent/imports", None).await.1["imports"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let (_, other) = h.connection("Other agent").await;
    assert_eq!(
        h.bearer(&other, &format!("/api/agent/imports/{import_id}"), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut smuggled = workplace_import();
    smuggled["execute"] = json!("pay everything");
    assert_eq!(
        h.bearer(&token, IMPORT, Some(smuggled)).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        h.bearer(
            &token,
            IMPORT,
            Some(json!({"items":[{"kind":"payment","title":"x"}]}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        h.owner("/api/agents/connections/revoke", Some(json!({"id":id})))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer(
            &token,
            IMPORT,
            Some(json!({"items":[{"kind":"note","title":"x"}]}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn imported_resources_and_relations_enter_the_ontology_only_after_approval() {
    let h = Harness::new().await;
    let (_, token) = h.connection("Dify").await;
    let outcome = h.bearer(&token, IMPORT, Some(workplace_import())).await.1;
    let proposals = proposals_of(&h, &outcome["import_id"]).await;
    let by_action = |action: &str, kind: &str| {
        proposals
            .iter()
            .find(|p| {
                p["properties"]["action"] == action
                    && (kind.is_empty() || p["properties"]["entity"]["kind"] == kind)
            })
            .unwrap()
            .clone()
    };
    let relation = by_action("create_relation", "");
    assert_eq!(relation["properties"]["relation"]["subject"], "#p1");
    let (status, error) = approve(&h, &relation, "").await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    let (status, created) =
        approve(&h, &by_action("create_entity", "resource"), "会议室投影仪").await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let resource = h.entity(created["entity_id"].as_str().unwrap()).await;
    assert_eq!(resource["title"], "会议室投影仪");
    assert_eq!(resource["scenes"], json!(["work"]));
    assert_eq!(resource["properties"]["unit"], "台");
    let (status, person) = approve(&h, &by_action("create_entity", "person"), "").await;
    assert_eq!(status, StatusCode::OK);
    let (status, linked) = approve(&h, &relation, "").await;
    assert_eq!(status, StatusCode::OK, "{linked}");
    let relations = h.relations().await;
    assert!(relations.iter().any(|r| r["subject"] == person["entity_id"]
        && r["predicate"] == "references"
        && r["object"] == created["entity_id"]));
    assert!(relations
        .iter()
        .any(|r| r["subject"] == created["entity_id"] && r["predicate"] == "derived_from"));
    let (status, task) = approve(&h, &by_action("create_task", ""), "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.entity(task["task_id"].as_str().unwrap()).await["title"],
        "跟进：周五前提交周报"
    );
    let import = h.entity(outcome["import_id"].as_str().unwrap()).await;
    assert_eq!(import["status"], "processed");
    assert_eq!(import["properties"]["route"]["executor"], "offline-rules");
}

#[tokio::test]
async fn chat_message_imports_become_follow_up_proposals() {
    let h = Harness::new().await;
    let (_, token) = h.connection("ChatGPT").await;
    let (status, outcome) = h
        .bearer(
            &token,
            IMPORT,
            Some(json!({"messages":[
                {"role":"system","content":"You are a helpful assistant"},
                {"role":"user","content":"提醒我周三带体检报告"}
            ]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{outcome}");
    let proposals = proposals_of(&h, &outcome["import_id"]).await;
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0]["title"], "跟进：提醒我周三带体检报告");
}

#[tokio::test]
async fn model_routes_are_loopback_only_and_run_by_a_leased_executor() {
    let h = Harness::new().await;
    for url in [
        "http://192.168.1.5:11434",
        "https://127.0.0.1:11434",
        "http://example.com",
    ] {
        assert_eq!(
            h.owner(
                "/api/agents/models",
                Some(json!({"title":"remote","provider":"ollama","base_url":url,"model":"m"}))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST,
            "{url}"
        );
    }
    let (status, model) = h
        .owner("/api/agents/models", Some(json!({"title":"本地 Qwen","provider":"ollama","base_url":"http://127.0.0.1:11434/","model":"qwen2.5:7b"})))
        .await;
    assert_eq!(status, StatusCode::OK, "{model}");
    assert_eq!(model["properties"]["base_url"], "http://127.0.0.1:11434");
    let (status, _) = h
        .owner("/api/agents/routes", Some(json!({"title":"工作导入走本地模型","priority":10,"scene":"work","executor":model["id"]})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.owner(
            "/api/agents/routes",
            Some(json!({"title":"bad","executor":"no-such-model"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (_, token) = h.connection("n8n").await;
    let offline = h
        .bearer(
            &token,
            IMPORT,
            Some(json!({"scenes":["life"],"items":[{"kind":"note","title":"家务分工"}]})),
        )
        .await
        .1;
    assert_eq!(offline["status"], "processed");
    let queued = h
        .bearer(
            &token,
            IMPORT,
            Some(json!({"scenes":["work"],"items":[{"kind":"message","title":"整理客户反馈"}]})),
        )
        .await
        .1;
    assert_eq!(queued["status"], "queued");
    assert_eq!(queued["executor"], "local-model");
    let (_, vault) = h.core(0).await;
    let (_, directive) = h.owner("/api/vault/publish", Some(json!({"password":PASSWORD,"confirmed":true,"expected_version":1,"id":vault["items"][0]["id"],"title":"专注深度工作","brief":"每天保留两小时不被打断的时间"}))).await;
    h.owner(
        "/api/self/identities",
        Some(json!({"statement":"我是工程师","category":"social","agent_visible":true})),
    )
    .await;
    h.owner(
        "/api/self/identities",
        Some(json!({"statement":"HIDDEN-SELF","category":"social"})),
    )
    .await;
    assert_eq!(
        h.bearer(&token, "/api/agent/jobs/claim", Some(json!({})))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (status, claim) = h
        .agent(
            "/api/agent/jobs/claim",
            Some(json!({"worker":"test-worker"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{claim}");
    let job = claim["job"].clone();
    assert_eq!(job["model"]["model"], "qwen2.5:7b");
    assert_eq!(job["import"]["items"][0]["title"], "整理客户反馈");
    let text = job.to_string();
    for expected in ["每天保留两小时", "我是工程师", "prm-candidates-v1"] {
        assert!(text.contains(expected), "{expected}");
    }
    assert!(!text.contains("HIDDEN-SELF") && !text.contains("PRIVATE-CORE"));
    assert!(h.agent("/api/agent/jobs/claim", Some(json!({}))).await.1["job"].is_null());
    let run = job["run_id"].as_str().unwrap();
    let result = format!("/api/agent/jobs/{run}/result");
    assert_eq!(
        h.agent(&result, Some(json!({"lease":"wrong","candidates":[]})))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.agent(&result, Some(json!({"lease":job["lease"],"candidates":[{"action":"create_task","title":"x","rationale":"y","command":"pay"}]})))
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, staged) = h
        .agent(&result, Some(json!({"lease":job["lease"],"summary":"整理出一项任务","candidates":[
            {"ref":"t1","action":"create_task","title":"汇总客户反馈","rationale":"与“专注深度工作”一致：集中处理一次。","principles":[directive["id"]],"scenes":["work"]},
            {"action":"create_entity","kind":"cash_entry","title":"付款","rationale":"x"},
            {"action":"create_relation","subject":"#missing","predicate":"references","object":"#t1","rationale":"x"}
        ]})))
        .await;
    assert_eq!(status, StatusCode::OK, "{staged}");
    assert_eq!(staged["proposals_created"], 1);
    assert_eq!(staged["rejected"].as_array().unwrap().len(), 2);
    let proposal = proposals_of(&h, &queued["import_id"]).await[0].clone();
    assert_eq!(proposal["properties"]["provider"], "local:qwen2.5:7b");
    assert_eq!(
        proposal["properties"]["principles"][0]["title"],
        "专注深度工作"
    );
    assert_eq!(
        h.agent(&result, Some(json!({"lease":job["lease"],"candidates":[]})))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.entity(queued["import_id"].as_str().unwrap()).await["status"],
        "processed"
    );
}

#[tokio::test]
async fn failed_runs_are_retried_then_marked_failed() {
    let h = Harness::new().await;
    let (_, model) = h
        .owner("/api/agents/models", Some(json!({"title":"LM Studio","provider":"openai_compatible","base_url":"http://localhost:1234/v1","model":"local-model"})))
        .await;
    h.owner(
        "/api/agents/routes",
        Some(json!({"title":"全部走本地模型","executor":model["id"]})),
    )
    .await;
    let (_, token) = h.connection("LangChain").await;
    let queued = h
        .bearer(
            &token,
            IMPORT,
            Some(json!({"items":[{"kind":"note","title":"会议纪要"}]})),
        )
        .await
        .1;
    for attempt in 1..=3 {
        let job = h.agent("/api/agent/jobs/claim", Some(json!({}))).await.1["job"].clone();
        assert_eq!(job["attempt"], attempt);
        let path = format!("/api/agent/jobs/{}/fail", job["run_id"].as_str().unwrap());
        assert_eq!(
            h.agent(
                &path,
                Some(json!({"lease":job["lease"],"error":"model offline"}))
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    assert!(h.agent("/api/agent/jobs/claim", Some(json!({}))).await.1["job"].is_null());
    assert_eq!(
        h.entity(queued["import_id"].as_str().unwrap()).await["status"],
        "failed"
    );
}

#[tokio::test]
async fn agent_connections_open_relations_and_link_their_imports() {
    let h = Harness::new().await;
    let (status, created) = h
        .owner(
            "/api/agents/connections",
            Some(json!({
                "name": "Grok 机器人", "protocol": "http", "platform": "grok",
                "relationship_type": "ai_assistant", "scenes": ["work", "learning"],
                "notes": "工作与学习助手"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let agent = h.entity(created["agent_id"].as_str().unwrap()).await;
    assert_eq!(agent["kind"], "ai_agent");
    assert_eq!(agent["worlds"], json!(["digital"]));
    assert_eq!(agent["properties"]["platform"], "grok");
    assert_eq!(agent["scenes"], json!(["work", "learning"]));
    let relationship = h.entity(created["relationship_id"].as_str().unwrap()).await;
    assert_eq!(relationship["kind"], "relationship");
    assert_eq!(relationship["properties"]["party_kind"], "ai_agent");
    assert_eq!(relationship["properties"]["agent_name"], "Grok 机器人");
    assert_eq!(relationship["scenes"], json!(["work", "learning"]));
    let relations = h.relations().await;
    assert!(relations.iter().any(|r| {
        r["subject"] == created["relationship_id"]
            && r["predicate"] == "involves"
            && r["object"] == created["agent_id"]
    }));
    assert!(relations.iter().any(|r| {
        r["subject"] == created["relationship_id"] && r["predicate"] == "relates_to_self"
    }));
    let token = created["token"].as_str().unwrap();
    let outcome = h
        .bearer(
            token,
            IMPORT,
            Some(json!({"items":[{"kind":"note","title":"跟进纪要"}]})),
        )
        .await
        .1;
    let import = h.entity(outcome["import_id"].as_str().unwrap()).await;
    assert_eq!(import["properties"]["agent_entity_id"], created["agent_id"]);
    assert!(h.relations().await.iter().any(|r| {
        r["subject"] == outcome["import_id"]
            && r["predicate"] == "from_agent"
            && r["object"] == created["agent_id"]
    }));
    let list = h.owner("/api/agents/connections", None).await.1;
    let entry = list["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == created["id"])
        .unwrap()
        .clone();
    assert_eq!(entry["platform"], "grok");
    assert_eq!(entry["relationship_id"], created["relationship_id"]);
    assert_eq!(entry["scenes"], json!(["work", "learning"]));
    assert!(list["platforms"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"] == "grok"));
    assert!(list["agent_relationship_types"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == "ai_assistant"));
}

#[tokio::test]
async fn agent_relations_stay_in_the_connection_flow() {
    let h = Harness::new().await;
    for body in [
        json!({"name":"x","protocol":"http","platform":"nope"}),
        json!({"name":"x","protocol":"http","relationship_type":"friend"}),
        json!({"name":"","protocol":"http"}),
        json!({"name":"x","protocol":"carrier-pigeon"}),
    ] {
        assert_eq!(
            h.owner("/api/agents/connections", Some(body.clone()))
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
    assert_eq!(
        h.owner(
            "/api/relationships",
            Some(json!({"person_name":"李华","type":"ai_assistant"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    // A connection without explicit fields stays backward compatible.
    let (id, token) = h.connection("测试 Agent").await;
    assert!(!id.is_empty() && token.starts_with("prm_"));
}

#[tokio::test]
async fn disabled_models_fall_back_to_offline_rules() {
    let h = Harness::new().await;
    let (_, model) = h
        .owner("/api/agents/models", Some(json!({"title":"Ollama","provider":"ollama","base_url":"http://127.0.0.1:11434","model":"llama3"})))
        .await;
    h.owner(
        "/api/agents/routes",
        Some(json!({"title":"全部走本地模型","executor":model["id"]})),
    )
    .await;
    assert_eq!(
        h.owner(
            &format!(
                "/api/agents/config/{}/status",
                model["id"].as_str().unwrap()
            ),
            Some(json!({"status":"disabled","expected_version":1}))
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, token) = h.connection("Coze").await;
    let outcome = h
        .bearer(
            &token,
            IMPORT,
            Some(json!({"items":[{"kind":"note","title":"读书清单"}]})),
        )
        .await
        .1;
    assert_eq!(outcome["status"], "processed");
    let import = h.entity(outcome["import_id"].as_str().unwrap()).await;
    assert_eq!(
        import["properties"]["route"]["note"],
        "规则指向的本地模型不可用，已改用离线规则"
    );
}
