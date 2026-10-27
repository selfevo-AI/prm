use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Bad(String),
    #[error("请先登录")]
    Unauthorized,
    #[error("此操作仅允许用户本人执行")]
    Forbidden,
    #[error("思维库已锁定，请重新解锁")]
    Locked,
    #[error("记录不存在")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("尝试次数过多，请 30 秒后重试")]
    RateLimited,
    #[error("数据库操作失败")]
    Db(#[from] postgres::Error),
    #[error("数据格式错误")]
    Json(#[from] serde_json::Error),
    #[error("无法读取加密数据")]
    Crypto,
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Bad(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Locked => StatusCode::LOCKED,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({"error": self.to_string()}))).into_response()
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Bad(message.into()))
    }
}
