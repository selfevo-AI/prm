mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};

async fn order_action(h: &Harness, order: &str, action: &str, body: Value) -> (StatusCode, Value) {
    h.owner(
        &format!("/api/purchase-orders/{order}/{action}"),
        Some(body),
    )
    .await
}

#[tokio::test]
async fn purchase_orders_follow_their_lifecycle_and_post_stock_and_cash() {
    let h = Harness::new().await;
    let supplier = h
        .create("supplier", "家居耗材店", json!({"contact":"王先生"}))
        .await;
    let room = h.create("location", "储物间", json!({"code":"A-01"})).await;
    let shelf = h
        .create(
            "location",
            "储物间 · 二层架",
            json!({"parent_id":room["id"]}),
        )
        .await;
    let filter = h
        .create("resource", "净水滤芯", json!({"unit":"个","location_id":shelf["id"],"minimum":2,"reorder_qty":4,"supplier_id":supplier["id"]}))
        .await;
    assert_eq!(filter["properties"]["location"], "储物间 · 二层架");
    assert_eq!(filter["properties"]["supplier"], "家居耗材店");
    let relations = h.relations().await;
    for (subject, predicate, object) in [
        (&filter, "stored_at", &shelf),
        (&supplier, "supplies", &filter),
        (&shelf, "located_in", &room),
    ] {
        assert!(
            relations.iter().any(|r| r["subject"] == subject["id"]
                && r["predicate"] == predicate
                && r["object"] == object["id"]),
            "{predicate}"
        );
    }
    let order = h
        .create(
            "purchase_order",
            "十月补货",
            json!({"supplier_id":supplier["id"],"expected_date":"2026-11-01"}),
        )
        .await;
    assert_eq!(order["status"], "draft");
    let id = order["id"].as_str().unwrap();
    assert_eq!(
        order_action(
            &h,
            id,
            "status",
            json!({"status":"ordered","expected_version":1})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let line = h
        .create("purchase_order_line", "滤芯 × 5", json!({"purchase_order_id":id,"resource_id":filter["id"],"quantity":5,"unit_cost_cents":6900}))
        .await;
    let current = h.entity(id).await;
    assert_eq!(current["properties"]["total_cents"], 34500);
    assert_eq!(current["version"], 2);
    assert!(h
        .relations()
        .await
        .iter()
        .any(|r| r["subject"] == line["id"]
            && r["predicate"] == "orders"
            && r["object"] == filter["id"]));
    assert_eq!(
        order_action(
            &h,
            id,
            "status",
            json!({"status":"ordered","expected_version":2})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.owner("/api/entities", Some(json!({"kind":"purchase_order_line","title":"late","properties":{"purchase_order_id":id,"resource_id":filter["id"],"quantity":1,"unit_cost_cents":1}})))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let receive = |quantity: i64, version: i64| json!({"lines":[{"line_id":line["id"],"quantity":quantity}],"expected_version":version});
    assert_eq!(
        order_action(&h, id, "receive", receive(6, 3)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        order_action(&h, id, "receive", receive(3, 3)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        order_action(&h, id, "receive", receive(1, 3)).await.0,
        StatusCode::CONFLICT
    );
    let partial = h.entity(id).await;
    assert_eq!(partial["status"], "partially_received");
    assert_eq!(
        h.entity(filter["id"].as_str().unwrap()).await["properties"]["quantity"],
        3
    );
    let movement = h.entities("stock_movement").await[0].clone();
    assert_eq!(movement["properties"]["movement_type"], "receipt");
    assert_eq!(movement["properties"]["purchase_order_line_id"], line["id"]);
    assert!(h
        .relations()
        .await
        .iter()
        .any(|r| r["subject"] == movement["id"]
            && r["predicate"] == "receives"
            && r["object"] == line["id"]));
    assert_eq!(
        order_action(
            &h,
            id,
            "status",
            json!({"status":"cancelled","expected_version":4})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let pay = |amount: i64, version: i64| json!({"amount_cents":amount,"date":"2026-10-27","expected_version":version});
    assert_eq!(
        order_action(&h, id, "pay", pay(50000, 4)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        order_action(&h, id, "pay", pay(34500, 4)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        order_action(&h, id, "pay", pay(1, 5)).await.0,
        StatusCode::BAD_REQUEST
    );
    let cash = h.entities("cash_entry").await[0].clone();
    assert_eq!(cash["properties"]["purchase_order_id"], id);
    assert_eq!(cash["properties"]["direction"], "expense");
    assert!(h
        .relations()
        .await
        .iter()
        .any(|r| r["subject"] == cash["id"] && r["predicate"] == "pays" && r["object"] == id));
    assert_eq!(
        order_action(&h, id, "receive", receive(2, 5)).await.0,
        StatusCode::OK
    );
    let received = h.entity(id).await;
    assert_eq!(received["status"], "received");
    assert_eq!(received["properties"]["paid_cents"], 34500);
    assert_eq!(
        h.entity(line["id"].as_str().unwrap()).await["status"],
        "received"
    );
    assert_eq!(
        order_action(
            &h,
            id,
            "status",
            json!({"status":"closed","expected_version":6})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(h.entity(id).await["status"], "closed");
}

#[tokio::test]
async fn reorder_and_warranty_rules_produce_reviewable_proposals() {
    let h = Harness::new().await;
    let supplier = h.create("supplier", "社区五金店", json!({})).await;
    h.create("resource", "灯泡", json!({"unit":"个","location":"阳台","minimum":3,"reorder_qty":6,"lead_time_days":2,"supplier_id":supplier["id"]}))
        .await;
    let warranty = (chrono::Local::now().date_naive() + chrono::Days::new(10)).to_string();
    h.create("resource", "洗衣机", json!({"unit":"台","location":"阳台","minimum":0,"item_type":"asset","warranty_until":warranty,"serial_no":"WM-01"}))
        .await;
    assert_eq!(
        h.owner("/api/entities", Some(json!({"kind":"resource","title":"x","properties":{"unit":"个","location":"y","minimum":0,"item_type":"service"}})))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        h.owner("/api/analyze", Some(json!({}))).await.1["proposals_created"],
        2
    );
    let proposals = h.entities("proposal").await;
    let low = proposals
        .iter()
        .find(|p| p["properties"]["rule"] == "low_stock")
        .unwrap();
    let reason = low["properties"]["rationale"].as_str().unwrap();
    assert!(
        reason.contains("建议补货量 6 个") && reason.contains("社区五金店"),
        "{reason}"
    );
    assert!(proposals
        .iter()
        .any(|p| p["properties"]["rule"] == "warranty" && p["title"] == "保修检查：洗衣机"));
    assert_eq!(
        h.owner("/api/analyze", Some(json!({}))).await.1["proposals_created"],
        0
    );
}

#[tokio::test]
async fn stock_movement_types_must_match_their_direction() {
    let h = Harness::new().await;
    let resource = h.resource().await;
    let path = format!(
        "/api/resources/{}/movement",
        resource["id"].as_str().unwrap()
    );
    for (delta, movement_type) in [(-1, "receipt"), (2, "issue"), (2, "gift")] {
        assert_eq!(
            h.owner(&path, Some(json!({"delta":delta,"reason":"x","expected_version":1,"movement_type":movement_type})))
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{movement_type}"
        );
    }
    assert_eq!(
        h.owner(&path, Some(json!({"delta":5,"reason":"盘点多出","expected_version":1,"movement_type":"adjustment"})))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.owner(&path, Some(json!({"delta":-2,"reason":"盘点短少","expected_version":2,"movement_type":"adjustment"})))
            .await
            .0,
        StatusCode::OK
    );
    let types: Vec<Value> = h
        .entities("stock_movement")
        .await
        .iter()
        .map(|m| m["properties"]["movement_type"].clone())
        .collect();
    assert_eq!(types, vec![json!("adjustment"), json!("adjustment")]);
    assert_eq!(
        h.entity(resource["id"].as_str().unwrap()).await["properties"]["quantity"],
        3
    );
}
