mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};

async fn assess(h: &Harness, body: Value) -> (StatusCode, Value) {
    h.owner("/api/self/assessments", Some(body)).await
}

#[tokio::test]
async fn self_assessments_follow_the_catalog_and_stay_separate_observations() {
    let h = Harness::new().await;
    let (status, catalog) = h.owner("/api/self/catalog", None).await;
    assert_eq!(status, StatusCode::OK);
    let rses = catalog["instruments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "rses")
        .unwrap()
        .clone();
    assert!(rses["citation"].as_str().unwrap().contains("Rosenberg"));
    assert_eq!(rses["metrics"][0]["max"], 40.0);
    let (status, body) = assess(
        &h,
        json!({"instrument":"bpnsfs","observed_at":"2026-10-20","scores":[
            {"metric":"bpnsfs.autonomy_satisfaction","value":4.25},
            {"metric":"bpnsfs.relatedness_frustration","value":1.5}
        ]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["recorded"], 2);
    for bad in [
        json!({"instrument":"rses","observed_at":"2026-10-20","scores":[{"metric":"rses.total","value":41}]}),
        json!({"instrument":"rses","observed_at":"2026-10-20","scores":[{"metric":"swls.total","value":20}]}),
        json!({"instrument":"ios","observed_at":"2026-10-20","scores":[{"metric":"ios.closeness","value":5}]}),
        json!({"instrument":"rses","observed_at":"2026-10-20","scores":[{"metric":"rses.total","value":30},{"metric":"rses.total","value":31}]}),
        json!({"instrument":"rses","observed_at":"yesterday","scores":[{"metric":"rses.total","value":30}]}),
        json!({"instrument":"rses","observed_at":"2026-10-20","scores":[]}),
    ] {
        assert_eq!(
            assess(&h, bad.clone()).await.0,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }
    assert_eq!(
        assess(&h, json!({"instrument":"rses","observed_at":"2026-10-20","scores":[{"metric":"rses.total","value":30}],"total_score":100})).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, _) = assess(&h, json!({"instrument":"custom","custom_label":"冥想时长","custom_unit":"分钟","custom_domain":"psychology","observed_at":"2026-10-21","scores":[{"metric":"custom","value":20}]})).await;
    assert_eq!(status, StatusCode::OK);
    let profile = h.owner("/api/self/profile", None).await.1;
    let indicators = profile["indicators"].as_array().unwrap();
    assert_eq!(indicators.len(), 3);
    assert!(indicators
        .iter()
        .all(|i| i["properties"]["agent_visible"] == false && i["kind"] == "psychological_metric"));
    let frustration = indicators
        .iter()
        .find(|i| i["properties"]["metric"] == "bpnsfs.relatedness_frustration")
        .unwrap();
    assert_eq!(frustration["properties"]["higher"], "worse");
    assert_eq!(frustration["scenes"], json!(["self", "work", "social"]));
    let profile_id = profile["profile"]["id"].clone();
    assert_eq!(
        h.relations()
            .await
            .iter()
            .filter(|r| r["predicate"] == "describes_self" && r["object"] == profile_id)
            .count(),
        3
    );
}

#[tokio::test]
async fn only_agent_visible_self_entries_reach_the_principles_context() {
    let h = Harness::new().await;
    let (status, _) = h
        .owner("/api/self/identities", Some(json!({"statement":"我是一名终身学习者","category":"reflective","centrality":5,"agent_visible":true})))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, hidden) = h
        .owner(
            "/api/self/identities",
            Some(json!({"statement":"HIDDEN-IDENTITY","category":"social"})),
        )
        .await;
    assert_eq!(
        h.owner(
            "/api/self/identities",
            Some(json!({"statement":"x","category":"unknown"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for (title, stance, visible) in [
        ("HIDDEN-STANCE 我对自己太苛刻", "criticism", false),
        ("我愿意善待失败时的自己", "compassion", true),
    ] {
        let (status, body) = h
            .owner(
                "/api/self/relations",
                Some(json!({"title":title,"stance":stance,"intensity":4,"agent_visible":visible})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    assert_eq!(
        h.owner(
            "/api/self/relations",
            Some(json!({"title":"x","stance":"diagnosis"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for (label, visible) in [("HIDDEN-METRIC", false), ("VISIBLE-METRIC", true)] {
        let (status, _) = assess(&h, json!({"instrument":"custom","custom_label":label,"observed_at":"2026-10-21","scores":[{"metric":"custom","value":7}],"agent_visible":visible})).await;
        assert_eq!(status, StatusCode::OK);
    }
    let preview = h.owner("/api/self/agent-preview", None).await.1.to_string();
    for expected in [
        "终身学习者",
        "善待失败",
        "VISIBLE-METRIC",
        "prm-principles-v1",
    ] {
        assert!(preview.contains(expected), "{expected}");
    }
    assert!(!preview.contains("HIDDEN"));
    let context = h.agent("/api/agent/context", None).await.1.to_string();
    assert!(context.contains("终身学习者") && !context.contains("HIDDEN"));
    let path = "/api/self/visibility";
    assert_eq!(
        h.owner(
            path,
            Some(json!({"id":hidden["id"],"agent_visible":true,"expected_version":1}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert!(h
        .owner("/api/self/agent-preview", None)
        .await
        .1
        .to_string()
        .contains("HIDDEN-IDENTITY"));
    assert_eq!(
        h.owner(
            path,
            Some(json!({"id":hidden["id"],"agent_visible":false,"expected_version":1}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let resource = h.resource().await;
    assert_eq!(
        h.owner(
            path,
            Some(json!({"id":resource["id"],"agent_visible":true,"expected_version":1}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn relationships_keep_an_interaction_ledger_and_surface_due_contacts() {
    let h = Harness::new().await;
    let (status, rel) = h
        .owner("/api/relationships", Some(json!({"person_name":"李华","type":"friend","closeness":5,"satisfaction":4.2,"contact_interval_days":14})))
        .await;
    assert_eq!(status, StatusCode::OK, "{rel}");
    assert_eq!(rel["title"], "李华 · 朋友");
    assert_eq!(rel["scenes"], json!(["social"]));
    let relations = h.relations().await;
    for predicate in ["involves", "relates_to_self"] {
        assert!(relations
            .iter()
            .any(|r| r["subject"] == rel["id"] && r["predicate"] == predicate));
    }
    for bad in [
        json!({"person_name":"x","type":"friend","closeness":8}),
        json!({"person_name":"x","type":"enemy"}),
        json!({"person_name":"x","type":"friend","satisfaction":6}),
        json!({"person_name":"","type":"friend"}),
    ] {
        assert_eq!(
            h.owner("/api/relationships", Some(bad.clone())).await.0,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }
    let id = rel["id"].as_str().unwrap();
    let path = format!("/api/relationships/{id}/interactions");
    let today = chrono::Local::now().date_naive();
    let month_ago = (today - chrono::Days::new(30)).to_string();
    let two_months_ago = (today - chrono::Days::new(60)).to_string();
    let (status, _) = h
        .owner(&path, Some(json!({"date":month_ago,"channel":"meet","valence":2,"notes":"一起吃饭","expected_version":1})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.owner(
            &path,
            Some(json!({"date":month_ago,"channel":"meet","expected_version":1}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let tomorrow_plus = (today + chrono::Days::new(3)).to_string();
    assert_eq!(
        h.owner(
            &path,
            Some(json!({"date":tomorrow_plus,"channel":"call","expected_version":2}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, _) = h
        .owner(
            &path,
            Some(json!({"date":two_months_ago,"channel":"call","valence":-1,"expected_version":2})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let current = h.entity(id).await;
    assert_eq!(current["properties"]["last_contact_at"], month_ago);
    assert_eq!(h.entities("interaction").await.len(), 2);
    let analysis = h.owner("/api/analyze", Some(json!({}))).await.1;
    assert_eq!(analysis["proposals_created"], 1);
    let proposal = h
        .entities("proposal")
        .await
        .into_iter()
        .find(|p| p["properties"]["rule"] == "contact_due")
        .unwrap();
    assert_eq!(proposal["title"], "联系：李华");
    assert_eq!(
        h.owner("/api/analyze", Some(json!({}))).await.1["proposals_created"],
        0
    );
    let (status, outcome) = h
        .owner(
            &format!("/api/proposals/{}/decide", proposal["id"].as_str().unwrap()),
            Some(json!({"decision":"approve","expected_version":1})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let task = h.entity(outcome["task_id"].as_str().unwrap()).await;
    assert_eq!(task["scenes"], json!(["social"]));
    let (status, _) = h
        .owner(
            &format!("/api/relationships/{id}"),
            Some(json!({"closeness":6,"contact_interval_days":14,"status":"paused","expected_version":3,"scenes":["emotion"]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let paused = h.entity(id).await;
    assert_eq!(paused["status"], "paused");
    assert_eq!(paused["scenes"], json!(["emotion"]));
    assert_eq!(
        h.owner("/api/analyze", Some(json!({}))).await.1["proposals_created"],
        0
    );
}

#[tokio::test]
async fn scenes_are_validated_stored_and_exported() {
    let h = Harness::new().await;
    let (status, task) = h
        .owner("/api/entities", Some(json!({"kind":"task","title":"准备述职","scenes":["work","learning"],"properties":{}})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(task["scenes"], json!(["work", "learning"]));
    assert_eq!(
        h.owner(
            "/api/entities",
            Some(json!({"kind":"task","title":"x","scenes":["moon"],"properties":{}}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let note = h.create("note", "读书笔记", json!({"body":"笔记"})).await;
    assert_eq!(note["scenes"], json!(["learning"]));
    let export = h.owner("/api/export", None).await.1;
    assert_eq!(export["format"], "prm-operations-v2");
    assert!(export["ontology"]["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"] == "emotion"));
    assert!(export["entities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["scenes"] == json!(["work", "learning"])));
}
