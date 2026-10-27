use crate::{
    error::{require, Error, Result},
    params,
    store::{id, now},
    Actor, AppState,
};
use axum::{extract::State, Extension, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn public_key(value: &str) -> Result<VerifyingKey> {
    let bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| Error::Forbidden)?
        .try_into()
        .map_err(|_| Error::Forbidden)?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| Error::Forbidden)?;
    if key.is_weak() {
        return Err(Error::Forbidden);
    }
    Ok(key)
}
pub fn verify(
    key: &VerifyingKey,
    method: &str,
    path: &str,
    timestamp: &str,
    nonce: &str,
    body: &[u8],
    signature: &str,
) -> Result<()> {
    let at: i64 = timestamp.parse().map_err(|_| Error::Forbidden)?;
    if chrono::Utc::now().timestamp().abs_diff(at) > 60
        || nonce.len() < 22
        || nonce.len() > 128
        || !nonce
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(Error::Forbidden);
    }
    let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(body));
    let message = format!("PRM-REQUEST-V1\n{method}\n{path}\n{timestamp}\n{nonce}\n{digest}");
    let signature = Signature::from_slice(
        &URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| Error::Forbidden)?,
    )
    .map_err(|_| Error::Forbidden)?;
    key.verify_strict(message.as_bytes(), &signature)
        .map_err(|_| Error::Forbidden)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    password: zeroize::Zeroizing<String>,
    public_key: String,
    label: String,
    item_ids: Vec<String>,
    expires_in_seconds: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revoke {
    id: String,
}
pub async fn grants(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    let mut stmt = inner.store.db.prepare("SELECT id,public_key,label,item_ids,expires_at,revoked,created_at FROM read_grants ORDER BY created_at DESC")?;
    let rows = stmt.query_map(&[], |r| Ok(json!({"id":r.get::<_,String>(0)?,"public_key":r.get::<_,String>(1)?,"label":r.get::<_,String>(2)?,"item_ids":r.get::<_,String>(3)?,"expires_at":r.get::<_,i64>(4)?,"revoked":r.get::<_,bool>(5)?,"created_at":r.get::<_,String>(6)?,"permission":"read"})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(Json(json!({"grants":rows})))
}
pub async fn grant(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Grant>,
) -> Result<Json<Value>> {
    let token = actor.owner()?;
    public_key(&body.public_key)?;
    crate::ontology::validate_text(&body.label, 120)?;
    require(
        (60..=86400).contains(&body.expires_in_seconds),
        "授权有效期应为 1 分钟至 24 小时",
    )?;
    require(
        !body.item_ids.is_empty() && body.item_ids.len() <= 100,
        "请选择 1 至 100 个条目",
    )?;
    let mut inner = state.lock()?;
    inner.check_password(&body.password)?;
    let key = inner
        .sessions
        .get(token)
        .and_then(|s| s.vault_key.as_ref())
        .ok_or(Error::Locked)?;
    let vault = inner.vault_with_key(key)?;
    require(
        body.item_ids
            .iter()
            .all(|id| vault.items.iter().any(|i| &i.id == id)),
        "授权条目不存在",
    )?;
    let grant_id = id();
    let expiry = chrono::Utc::now().timestamp() + body.expires_in_seconds;
    inner.store.transaction(|store| {
        store.db.execute("INSERT INTO read_grants(id,public_key,label,item_ids,expires_at,created_at) VALUES($1,$2,$3,$4,$5,$6)",params![grant_id,body.public_key,body.label,serde_json::to_string(&body.item_ids)?,expiry,now()])?;
        store.audit("owner","vault.read_granted",&grant_id)
    })?;
    Ok(Json(
        json!({"id":grant_id,"expires_at":expiry,"permission":"read"}),
    ))
}
pub async fn revoke(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Revoke>,
) -> Result<Json<Value>> {
    actor.owner()?;
    let inner = state.lock()?;
    inner.store.transaction(|store| {
        if store.db.execute(
            "UPDATE read_grants SET revoked=TRUE WHERE id=$1 AND NOT revoked",
            params![body.id],
        )? != 1
        {
            return Err(Error::NotFound);
        }
        store.audit("owner", "vault.read_revoked", &body.id)
    })?;
    Ok(Json(json!({"ok":true})))
}
pub async fn shared(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    let Actor::Reader(grant_id) = actor else {
        return Err(Error::Forbidden);
    };
    let inner = state.lock()?;
    // Recheck inside the same lock as decryption, so revocation cannot race a read.
    let (ids, expiry, revoked): (String, i64, bool) = inner.store.db.query_row(
        "SELECT item_ids,expires_at,revoked FROM read_grants WHERE id=$1",
        params![grant_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    if revoked || expiry <= chrono::Utc::now().timestamp() {
        return Err(Error::Forbidden);
    }
    let key = inner
        .sessions
        .values()
        .find_map(|s| {
            if s.vault_until > std::time::Instant::now() {
                s.vault_key.as_ref()
            } else {
                None
            }
        })
        .ok_or(Error::Locked)?;
    let vault = inner.vault_with_key(key)?;
    let ids: Vec<String> = serde_json::from_str(&ids)?;
    let items: Vec<_> = vault
        .items
        .into_iter()
        .filter(|i| ids.contains(&i.id))
        .collect();
    inner
        .store
        .audit("reader", "vault.shared_read", &grant_id)?;
    Ok(Json(json!({"permission":"read","items":items})))
}
