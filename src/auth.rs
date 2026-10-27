use crate::params;
use crate::{
    crypto,
    error::{require, Error, Result},
    vault::Vault,
    Actor, AppState, Inner, Session,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Password {
    pub password: Zeroizing<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Login {
    pub password: Zeroizing<String>,
    pub public_key: String,
}
impl Inner {
    pub fn check_password(&mut self, password: &str) -> Result<()> {
        if self
            .blocked_until
            .is_some_and(|until| until > Instant::now())
        {
            return Err(Error::RateLimited);
        }
        let (hash, _) = self.store.credentials()?;
        if !crypto::verify(password, &hash) {
            self.failures += 1;
            self.store.audit("anonymous", "auth.failed", "account")?;
            if self.failures >= 5 {
                self.blocked_until = Some(Instant::now() + Duration::from_secs(30));
                self.failures = 0;
            }
            return Err(Error::Unauthorized);
        }
        self.failures = 0;
        self.blocked_until = None;
        Ok(())
    }
    fn issue_session(&mut self, verifying_key: ed25519_dalek::VerifyingKey) -> Result<HeaderMap> {
        let token = crypto::token();
        // A single owner session keeps lock/revocation behaviour predictable across tabs.
        self.sessions.clear();
        self.sessions.insert(
            token.clone(),
            Session {
                verifying_key,
                touched: Instant::now(),
                created: Instant::now(),
                vault_key: None,
                vault_until: Instant::now(),
            },
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&format!(
                "prm_session={token}; HttpOnly; SameSite=Strict; Path=/"
            ))
            .map_err(|_| Error::Crypto)?,
        );
        Ok(headers)
    }
}
pub async fn status(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    let inner = state.lock()?;
    Ok(Json(
        json!({"initialized":inner.store.initialized()?,"authenticated":matches!(actor,Actor::Owner(_)),"mode":"offline-rules"}),
    ))
}
pub async fn setup(
    State(state): State<AppState>,
    Json(body): Json<Login>,
) -> Result<(HeaderMap, Json<Value>)> {
    require(
        body.password.chars().count() >= 12 && body.password.len() <= 512,
        "口令至少需要 12 个字符，且不超过 512 字节",
    )?;
    let verifying_key = crate::security::public_key(&body.public_key)?;
    let mut inner = state.lock()?;
    if inner.store.initialized()? {
        return Err(Error::Conflict("账户已创建".into()));
    }
    let salt = crypto::random::<16>();
    let hash = crypto::hash_password(&body.password)?;
    let key = crypto::derive(&body.password, &salt)?;
    let bytes = Zeroizing::new(serde_json::to_vec(&Vault::default())?);
    let sealed = crypto::encrypt(&key, &bytes)?;
    inner.store.transaction(|store| {
        store.db.execute(
            "INSERT INTO vault(id,payload) VALUES(1,$1)",
            params![sealed],
        )?;
        store.db.execute(
            "INSERT INTO account(id,password_hash,vault_salt) VALUES(1,$1,$2)",
            params![hash, salt.as_slice()],
        )?;
        for name in ["收件箱", "学习与研究", "日常协同"] {
            store.insert("channel", name, &[], json!({}), "active")?;
        }
        store.audit("owner", "account.created", "account")
    })?;
    Ok((
        inner.issue_session(verifying_key)?,
        Json(json!({"ok":true})),
    ))
}
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<Login>,
) -> Result<(HeaderMap, Json<Value>)> {
    let verifying_key = crate::security::public_key(&body.public_key)?;
    let mut inner = state.lock()?;
    inner.check_password(&body.password)?;
    inner.store.audit("owner", "auth.login", "account")?;
    Ok((
        inner.issue_session(verifying_key)?,
        Json(json!({"ok":true})),
    ))
}
pub async fn logout(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<(HeaderMap, Json<Value>)> {
    let token = actor.owner()?;
    let mut inner = state.lock()?;
    inner.sessions.remove(token);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static("prm_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"),
    );
    Ok((headers, Json(json!({"ok":true}))))
}
