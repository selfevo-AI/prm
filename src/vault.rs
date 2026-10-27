use crate::params;
use crate::{
    auth::Password,
    crypto,
    error::{require, Error, Result},
    ontology::{validate_scenes, validate_text, validate_values},
    store::{id, now},
    Actor, AppState, Inner,
};
use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize, Default)]
pub struct Vault {
    pub version: i64,
    pub items: Vec<CoreItem>,
    pub history: Vec<CoreItem>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CoreItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub values: Vec<String>,
    pub version: i64,
    pub updated_at: String,
    pub confirmed_at: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    pub password: Zeroizing<String>,
    pub confirmed: bool,
    pub expected_version: i64,
    pub id: Option<String>,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub values: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub password: Zeroizing<String>,
    pub confirmed: bool,
    pub expected_version: i64,
    pub id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publish {
    pub password: Zeroizing<String>,
    pub confirmed: bool,
    pub expected_version: i64,
    pub id: String,
    pub title: String,
    pub brief: String,
    /// Scenes the published direction applies to; empty means all scenes.
    #[serde(default)]
    pub scenes: Vec<String>,
}
impl Inner {
    pub(crate) fn vault_with_key(&self, key: &[u8; 32]) -> Result<Vault> {
        let payload: Vec<u8> =
            self.store
                .db
                .query_row("SELECT payload FROM vault WHERE id=1", &[], |r| r.get(0))?;
        Ok(serde_json::from_slice(&crypto::decrypt(key, &payload)?)?)
    }
    fn reauth_vault(
        &mut self,
        password: &str,
        confirmed: bool,
        expected: i64,
    ) -> Result<(Zeroizing<[u8; 32]>, Vault)> {
        require(confirmed, "需要用户本人明确确认")?;
        self.check_password(password)?;
        let (_, salt) = self.store.credentials()?;
        let key = crypto::derive(password, &salt)?;
        let vault = self.vault_with_key(&key)?;
        if vault.version != expected {
            return Err(Error::Conflict("思维库已有新版本，请重新打开后确认".into()));
        }
        Ok((key, vault))
    }
    fn write_vault(&self, key: &[u8; 32], vault: &Vault, action: &str) -> Result<()> {
        let bytes = Zeroizing::new(serde_json::to_vec(vault)?);
        let sealed = crypto::encrypt(key, &bytes)?;
        self.store.transaction(|store| {
            store
                .db
                .execute("UPDATE vault SET payload=$1 WHERE id=1", params![sealed])?;
            store.audit("owner", action, "vault")
        })?;
        Ok(())
    }
}
pub async fn unlock(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Password>,
) -> Result<Json<Vault>> {
    let token = actor.owner()?;
    let mut inner = state.lock()?;
    inner.check_password(&body.password)?;
    let (_, salt) = inner.store.credentials()?;
    let key = crypto::derive(&body.password, &salt)?;
    let vault = inner.vault_with_key(&key)?;
    let session = inner.sessions.get_mut(token).ok_or(Error::Unauthorized)?;
    session.vault_key = Some(key);
    session.vault_until = Instant::now() + Duration::from_secs(300);
    inner.store.audit("owner", "vault.unlocked", "vault")?;
    Ok(Json(vault))
}
pub async fn lock(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Value>> {
    let token = actor.owner()?;
    let mut inner = state.lock()?;
    if let Some(s) = inner.sessions.get_mut(token) {
        s.vault_key = None;
        s.vault_until = Instant::now();
    }
    Ok(Json(json!({"ok":true})))
}
pub async fn read(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<Vault>> {
    let token = actor.owner()?;
    let inner = state.lock()?;
    let key = inner
        .sessions
        .get(token)
        .and_then(|s| s.vault_key.as_ref())
        .ok_or(Error::Locked)?;
    Ok(Json(inner.vault_with_key(key)?))
}
pub async fn save(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Save>,
) -> Result<Json<Vault>> {
    actor.owner()?;
    validate_text(&body.title, 120)?;
    validate_text(&body.body, 12000)?;
    validate_values(&body.values)?;
    require(
        ["principle", "goal", "reflection"].contains(&body.kind.as_str()),
        "思维条目类型不合法",
    )?;
    let mut inner = state.lock()?;
    let (key, mut vault) =
        inner.reauth_vault(&body.password, body.confirmed, body.expected_version)?;
    let item_id = body.id.unwrap_or_else(id);
    let mut version = 1;
    if let Some(index) = vault.items.iter().position(|e| e.id == item_id) {
        let old = vault.items.remove(index);
        version = old.version + 1;
        vault.history.push(old);
    }
    vault.items.push(CoreItem {
        id: item_id.clone(),
        kind: body.kind,
        title: body.title.trim().into(),
        body: body.body,
        values: body.values,
        version,
        updated_at: now(),
        confirmed_at: now(),
    });
    vault.version += 1;
    inner.write_vault(&key, &vault, "vault.confirmed")?;
    Ok(Json(vault))
}
pub async fn review(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Review>,
) -> Result<Json<Vault>> {
    actor.owner()?;
    let mut inner = state.lock()?;
    let (key, mut vault) =
        inner.reauth_vault(&body.password, body.confirmed, body.expected_version)?;
    let item = vault
        .items
        .iter_mut()
        .find(|e| e.id == body.id)
        .ok_or(Error::NotFound)?;
    vault.history.push(item.clone());
    item.version += 1;
    item.confirmed_at = now();
    vault.version += 1;
    inner.write_vault(&key, &vault, "vault.reviewed")?;
    Ok(Json(vault))
}
pub async fn publish(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<Publish>,
) -> Result<Json<Value>> {
    actor.owner()?;
    validate_text(&body.title, 120)?;
    validate_text(&body.brief, 4000)?;
    validate_scenes(&body.scenes)?;
    let mut inner = state.lock()?;
    let (_, vault) = inner.reauth_vault(&body.password, body.confirmed, body.expected_version)?;
    let item = vault
        .items
        .iter()
        .find(|e| e.id == body.id)
        .ok_or(Error::NotFound)?;
    let entity = inner.store.transaction(|store| {
        let entity = store.insert_scoped(
            "directive",
            &body.title,
            &item.values,
            &body.scenes,
            json!({"body":body.brief,"authorized_at":now(),"source":"owner-published"}),
            "active",
        )?;
        store.audit("owner", "directive.published", &entity.id)?;
        Ok(entity)
    })?;
    Ok(Json(json!(entity)))
}
