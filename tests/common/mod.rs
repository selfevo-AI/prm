#![allow(dead_code)] // Each integration-test crate uses a different subset of the helpers.
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use prm::{app, crypto, AppState};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tower::ServiceExt;

pub const PASSWORD: &str = "test-owner-password-12345";
/// The local model executor token (PRM_AGENT_TOKEN).
pub const AGENT: &str = "test-agent-restricted-token-1234567890";
pub struct Harness {
    pub dir: TempDir,
    pub state: AppState,
    pub router: Router,
    pub cookie: String,
}
impl Harness {
    pub async fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let state = AppState::open(dir.path(), Some(AGENT.into())).unwrap();
        let router = app(state.clone());
        let (status, _, cookie) = send(
            &router,
            "/api/auth/setup",
            Some(json!({"password":PASSWORD})),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        Self {
            dir,
            state,
            router,
            cookie: cookie.unwrap(),
        }
    }
    pub async fn owner(&self, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let (s, b, _) = send(&self.router, path, body, Some(&self.cookie), None).await;
        (s, b)
    }
    /// A request as the local model executor.
    pub async fn agent(&self, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.bearer(AGENT, path, body).await
    }
    pub async fn bearer(&self, token: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let (s, b, _) = send(&self.router, path, body, None, Some(token)).await;
        (s, b)
    }
    pub async fn unlock(&self) {
        assert_eq!(
            self.owner("/api/vault/unlock", Some(json!({"password":PASSWORD})))
                .await
                .0,
            StatusCode::OK
        );
    }
    pub async fn core(&self, version: i64) -> (StatusCode, Value) {
        self.owner("/api/vault/entries",Some(json!({"password":PASSWORD,"confirmed":true,"expected_version":version,"id":null,"kind":"principle","title":"PRIVATE-CORE-TITLE","body":"PRIVATE-CORE-CONTENT-never-export","values":["existence"]}))).await
    }
    pub async fn resource(&self) -> Value {
        let (s,e)=self.owner("/api/entities",Some(json!({"kind":"resource","title":"Water filter","values":["need"],"properties":{"unit":"piece","location":"home","minimum":2}}))).await;
        assert_eq!(s, StatusCode::OK, "{e}");
        e
    }
    pub async fn create(&self, kind: &str, title: &str, properties: Value) -> Value {
        let (s, e) = self
            .owner(
                "/api/entities",
                Some(json!({"kind": kind, "title": title, "properties": properties})),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{kind}: {e}");
        e
    }
    /// Issue an external agent credential; returns (id, token).
    pub async fn connection(&self, name: &str) -> (String, String) {
        let (s, body) = self
            .owner(
                "/api/agents/connections",
                Some(json!({"name": name, "protocol": "mcp"})),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{body}");
        (
            body["id"].as_str().unwrap().into(),
            body["token"].as_str().unwrap().into(),
        )
    }
    pub async fn entities(&self, kind: &str) -> Vec<Value> {
        let workspace = self.owner("/api/workspace", None).await.1;
        workspace["entities"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == kind)
            .cloned()
            .collect()
    }
    pub async fn entity(&self, id: &str) -> Value {
        let workspace = self.owner("/api/workspace", None).await.1;
        workspace["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == id)
            .cloned()
            .unwrap_or(Value::Null)
    }
    pub async fn relations(&self) -> Vec<Value> {
        self.owner("/api/workspace", None).await.1["relations"]
            .as_array()
            .unwrap()
            .clone()
    }
}
pub async fn send(
    router: &Router,
    path: &str,
    body: Option<Value>,
    cookie: Option<&str>,
    bearer: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let key = SigningKey::from_bytes(&[42; 32]);
    let mut body = body;
    if ["/api/auth/setup", "/api/auth/login"].contains(&path) {
        if let Some(ref mut value) = body {
            value["public_key"] = json!(URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()));
        }
    }
    let encoded = body.as_ref().map(|v| v.to_string()).unwrap_or_default();
    let method = if body.is_some() { "POST" } else { "GET" };
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce = crypto::token();
    let message = format!(
        "PRM-REQUEST-V1\n{method}\n{path}\n{timestamp}\n{nonce}\n{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(encoded.as_bytes()))
    );
    let signature = URL_SAFE_NO_PAD.encode(key.sign(message.as_bytes()).to_bytes());
    let mut request = Request::builder()
        .header("x-prm-timestamp", timestamp)
        .header("x-prm-nonce", nonce)
        .header("x-prm-signature", signature)
        .uri(path)
        .method(method)
        .header("host", "127.0.0.1:4317")
        .header("x-prm-request", "1")
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    if let Some(token) = bearer {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(
            request
                .body(body.map_or(Body::empty(), |v| Body::from(v.to_string())))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let cookie = response
        .headers()
        .get("set-cookie")
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&bytes)})),
        cookie,
    )
}
#[allow(clippy::too_many_arguments)] // Tests vary each signed request component independently.
pub fn signed_request(
    path: &str,
    method: &str,
    body: &str,
    key: &SigningKey,
    nonce: &str,
    at: i64,
    cookie: Option<&str>,
    grant: Option<&str>,
) -> Request<Body> {
    let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(body.as_bytes()));
    let message = format!("PRM-REQUEST-V1\n{method}\n{path}\n{at}\n{nonce}\n{digest}");
    let mut request = Request::builder()
        .uri(path)
        .method(method)
        .header("host", "127.0.0.1:4317")
        .header("x-prm-request", "1")
        .header("content-type", "application/json")
        .header("x-prm-timestamp", at.to_string())
        .header("x-prm-nonce", nonce)
        .header(
            "x-prm-signature",
            URL_SAFE_NO_PAD.encode(key.sign(message.as_bytes()).to_bytes()),
        );
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    if let Some(grant) = grant {
        request = request.header("x-prm-grant", grant);
    }
    request.body(Body::from(body.to_string())).unwrap()
}
pub async fn reader(
    h: &Harness,
    id: &str,
    key: &SigningKey,
    path: &str,
    method: &str,
) -> (StatusCode, Value) {
    let req = signed_request(
        path,
        method,
        if method == "GET" { "" } else { "{}" },
        key,
        &crypto::token(),
        chrono::Utc::now().timestamp(),
        None,
        Some(id),
    );
    let response = h.router.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
