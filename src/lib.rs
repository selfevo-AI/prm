mod agents;
mod analysis;
mod auth;
pub mod crypto;
pub mod database;
pub mod error;
mod erp;
mod legacy;
pub mod ontology;
mod operations;
mod relationships;
mod security;
mod self_model;
pub mod store;
mod vault;

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use error::{Error, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use store::Store;
use subtle::ConstantTimeEq;
use tower_http::services::{ServeDir, ServeFile};
use zeroize::Zeroizing;

pub struct Session {
    verifying_key: ed25519_dalek::VerifyingKey,
    touched: Instant,
    created: Instant,
    vault_key: Option<Zeroizing<[u8; 32]>>,
    vault_until: Instant,
}
pub struct Inner {
    pub store: Store,
    sessions: HashMap<String, Session>,
    executor_hash: Option<[u8; 32]>,
    failures: u32,
    blocked_until: Option<Instant>,
}
impl Inner {
    fn purge_expired(&mut self, now: Instant) {
        self.sessions.retain(|_, session| {
            now.duration_since(session.touched) < Duration::from_secs(900)
                && now.duration_since(session.created) < Duration::from_secs(28800)
        });
        for session in self.sessions.values_mut() {
            if session.vault_until <= now {
                session.vault_key = None;
            }
        }
    }
}
#[derive(Clone)]
pub struct AppState(pub Arc<Mutex<Inner>>);
impl AppState {
    /// `executor_token` authenticates the local model executor (PRM_AGENT_TOKEN).
    pub fn open(
        path: &Path,
        executor_token: Option<String>,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        if executor_token.as_ref().is_some_and(|s| s.len() < 32) {
            return Err("PRM_AGENT_TOKEN must contain at least 32 characters".into());
        }
        Ok(Self(Arc::new(Mutex::new(Inner {
            store: Store::open(path)?,
            sessions: HashMap::new(),
            executor_hash: executor_token.map(|s| Sha256::digest(s.as_bytes()).into()),
            failures: 0,
            blocked_until: None,
        }))))
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>> {
        self.0.lock().map_err(|_| Error::Crypto)
    }
    pub fn purge_expired_sessions(&self) -> Result<()> {
        self.lock()?.purge_expired(Instant::now());
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub enum Actor {
    Public,
    Owner(String),
    /// The local model executor process (PRM_AGENT_TOKEN).
    Executor,
    /// An owner-issued external agent credential, by credential id.
    External(String),
    Reader(String),
}
impl Actor {
    fn owner(&self) -> Result<&str> {
        match self {
            Self::Owner(token) => Ok(token),
            Self::Executor | Self::External(_) | Self::Reader(_) => Err(Error::Forbidden),
            Self::Public => Err(Error::Unauthorized),
        }
    }
    fn executor(&self) -> Result<()> {
        match self {
            Self::Executor => Ok(()),
            _ => Err(Error::Forbidden),
        }
    }
    fn external(&self) -> Result<&str> {
        match self {
            Self::External(id) => Ok(id),
            _ => Err(Error::Forbidden),
        }
    }
    /// Paths a bearer-authenticated actor may reach at all; handlers check again.
    fn allows(&self, path: &str) -> bool {
        match self {
            Self::Executor => path.starts_with("/api/agent/"),
            Self::External(_) => {
                path == "/api/agent/import"
                    || path == "/api/agent/imports"
                    || path.starts_with("/api/agent/imports/")
            }
            _ => true,
        }
    }
}
async fn authenticated_boundary(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 128 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return axum::http::StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };
    req = Request::from_parts(parts, axum::body::Body::from(bytes.clone()));
    let result = (|| -> Result<Actor> {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let hostname = host.split(':').next().unwrap_or("");
        if !["127.0.0.1", "localhost"].contains(&hostname) {
            return Err(Error::Forbidden);
        }
        if let Some(origin) = req.headers().get(header::ORIGIN) {
            if origin.to_str().unwrap_or("") != format!("http://{host}") {
                return Err(Error::Forbidden);
            }
        }
        if ![Method::GET, Method::HEAD].contains(req.method())
            && req
                .headers()
                .get("x-prm-request")
                .and_then(|v| v.to_str().ok())
                != Some("1")
        {
            return Err(Error::Forbidden);
        }
        let mut inner = state.lock()?;
        let now = Instant::now();
        inner.purge_expired(now);
        if let Some(grant) = req
            .headers()
            .get("x-prm-grant")
            .and_then(|h| h.to_str().ok())
        {
            if req.method() != Method::GET || req.uri().path() != "/api/shared/vault" {
                return Err(Error::Forbidden);
            }
            let (public_key, expiry, revoked): (String, i64, bool) = inner
                .store
                .db
                .query_row(
                    "SELECT public_key,expires_at,revoked FROM read_grants WHERE id=$1",
                    params![grant],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(|_| Error::Forbidden)?;
            if revoked || expiry <= chrono::Utc::now().timestamp() {
                return Err(Error::Forbidden);
            }
            check_signature(
                &mut inner,
                &req,
                &bytes,
                &security::public_key(&public_key)?,
            )?;
            return Ok(Actor::Reader(grant.into()));
        }
        if let Some(auth) = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        {
            let Some(bearer) = auth.strip_prefix("Bearer ") else {
                return Err(Error::Unauthorized);
            };
            let digest: [u8; 32] = Sha256::digest(bearer.as_bytes()).into();
            if inner
                .executor_hash
                .as_ref()
                .is_some_and(|h| bool::from(h.ct_eq(&digest)))
            {
                return Ok(Actor::Executor);
            }
            if let Some(id) = agents::credentials::authenticate(&inner.store, &digest)? {
                return Ok(Actor::External(id));
            }
            return Err(Error::Unauthorized);
        }
        let cookie = req
            .headers()
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = cookie
            .split(';')
            .find_map(|part| part.trim().strip_prefix("prm_session="));
        if let Some(token) = token {
            if let Some(session) = inner.sessions.get_mut(token) {
                let verifying_key = session.verifying_key;
                if req.uri().path().starts_with("/api/")
                    && !["/api/auth/status", "/api/auth/login", "/api/auth/setup"]
                        .contains(&req.uri().path())
                {
                    check_signature(&mut inner, &req, &bytes, &verifying_key)?;
                }
                inner
                    .sessions
                    .get_mut(token)
                    .ok_or(Error::Unauthorized)?
                    .touched = now;
                return Ok(Actor::Owner(token.into()));
            }
        }
        Ok(Actor::Public)
    })();
    let actor = match result {
        Ok(actor) => actor,
        Err(e) => return e.into_response(),
    };
    if !actor.allows(req.uri().path()) {
        return Error::Forbidden.into_response();
    }
    req.extensions_mut().insert(actor);
    next.run(req).await
}
async fn boundary(state: State<AppState>, req: Request, next: Next) -> Response {
    let mut response = authenticated_boundary(state, req, next).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    response
}
fn check_signature(
    inner: &mut Inner,
    req: &Request,
    body: &[u8],
    key: &ed25519_dalek::VerifyingKey,
) -> Result<()> {
    let header = |name: &str| {
        req.headers()
            .get(name)
            .and_then(|h| h.to_str().ok())
            .ok_or(Error::Forbidden)
    };
    let nonce = header("x-prm-nonce")?;
    security::verify(
        key,
        req.method().as_str(),
        req.uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/"),
        header("x-prm-timestamp")?,
        nonce,
        body,
        header("x-prm-signature")?,
    )?;
    let identity = format!("{:?}:{nonce}", key.as_bytes());
    let now = chrono::Utc::now().timestamp();
    inner.store.db.execute(
        "DELETE FROM request_nonces WHERE expires_at < $1",
        params![now],
    )?;
    let expiry = now + 125;
    if inner.store.db.execute(
        "INSERT INTO request_nonces(id,expires_at) VALUES($1,$2) ON CONFLICT DO NOTHING",
        params![identity, expiry],
    )? != 1
    {
        return Err(Error::Forbidden);
    }
    Ok(())
}
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/status", get(auth::status))
        .route("/api/auth/setup", post(auth::setup))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/vault", get(vault::read))
        .route(
            "/api/vault/grants",
            get(security::grants).post(security::grant),
        )
        .route("/api/vault/grants/revoke", post(security::revoke))
        .route("/api/shared/vault", get(security::shared))
        .route("/api/vault/unlock", post(vault::unlock))
        .route("/api/vault/lock", post(vault::lock))
        .route("/api/vault/entries", post(vault::save))
        .route("/api/vault/review", post(vault::review))
        .route("/api/vault/publish", post(vault::publish))
        .route("/api/workspace", get(operations::workspace))
        .route("/api/entities", post(operations::create))
        .route(
            "/api/directives/{id}/revoke",
            post(operations::revoke_directive),
        )
        .route("/api/tasks/{id}/status", post(operations::task_status))
        .route("/api/relations", post(operations::relate))
        .route("/api/demo", post(operations::demo))
        .route("/api/export", get(operations::export))
        .route("/api/self/catalog", get(self_model::catalog))
        .route("/api/self/profile", get(self_model::profile))
        .route("/api/self/assessments", post(self_model::assessment))
        .route("/api/self/identities", post(self_model::identity))
        .route("/api/self/relations", post(self_model::self_relation))
        .route("/api/self/visibility", post(self_model::visibility))
        .route("/api/self/agent-preview", get(self_model::agent_preview))
        .route("/api/relationships", post(relationships::create))
        .route("/api/relationships/{id}", post(relationships::update))
        .route(
            "/api/relationships/{id}/interactions",
            post(relationships::interaction),
        )
        .route("/api/resources/{id}/movement", post(erp::movement))
        .route("/api/cash", post(erp::cash))
        .route("/api/purchase-orders/{id}/status", post(erp::order_status))
        .route("/api/purchase-orders/{id}/receive", post(erp::receive))
        .route("/api/purchase-orders/{id}/pay", post(erp::pay))
        .route("/api/messages", post(analysis::message))
        .route("/api/analyze", post(analysis::analyze))
        .route("/api/proposals/{id}/decide", post(analysis::decide))
        .route(
            "/api/agents/connections",
            get(agents::credentials::list).post(agents::credentials::create),
        )
        .route(
            "/api/agents/connections/revoke",
            post(agents::credentials::revoke),
        )
        .route("/api/agents/models", post(agents::router::create_model))
        .route("/api/agents/routes", post(agents::router::create_route))
        .route(
            "/api/agents/config/{id}/status",
            post(agents::router::config_status),
        )
        .route("/api/agent/import", post(agents::gateway::import))
        .route("/api/agent/imports", get(agents::gateway::imports))
        .route("/api/agent/imports/{id}", get(agents::gateway::import_status))
        .route("/api/agent/context", get(agents::executor::context))
        .route("/api/agent/proposals", post(agents::executor::proposal))
        .route("/api/agent/jobs/claim", post(agents::executor::claim))
        .route("/api/agent/jobs/{id}/result", post(agents::executor::result))
        .route("/api/agent/jobs/{id}/fail", post(agents::executor::fail))
        .fallback_service(
            ServeDir::new("dist").not_found_service(ServeFile::new("dist/index.html")),
        )
        .layer(axum::extract::DefaultBodyLimit::max(128 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state)
}

#[cfg(test)]
mod timeout_tests {
    use super::*;

    #[test]
    fn expiring_vault_keys_and_sessions_does_not_require_a_user_request() {
        let directory = tempfile::tempdir().unwrap();
        let state = AppState::open(directory.path(), None).unwrap();
        let start = Instant::now();
        let mut inner = state.lock().unwrap();
        inner.sessions.insert(
            "owner".into(),
            Session {
                verifying_key: ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key(),
                touched: start,
                created: start,
                vault_key: Some(Zeroizing::new([7; 32])),
                vault_until: start + Duration::from_secs(300),
            },
        );
        inner.purge_expired(start + Duration::from_secs(300));
        assert!(inner.sessions["owner"].vault_key.is_none());
        inner.purge_expired(start + Duration::from_secs(900));
        assert!(inner.sessions.is_empty());
    }

    #[test]
    fn bearer_actors_are_confined_to_their_paths() {
        assert!(Actor::Executor.allows("/api/agent/jobs/claim"));
        assert!(!Actor::Executor.allows("/api/agents/connections"));
        assert!(!Actor::Executor.allows("/api/workspace"));
        let external = Actor::External("a".into());
        assert!(external.allows("/api/agent/import"));
        assert!(external.allows("/api/agent/imports/x"));
        assert!(!external.allows("/api/agent/context"));
        assert!(!external.allows("/api/agent/jobs/claim"));
        assert!(!external.allows("/api/vault"));
    }
}
