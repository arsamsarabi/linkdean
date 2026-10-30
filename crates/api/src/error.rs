use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use linkdean_core::Error as CoreError;
use serde_json::json;

pub struct ApiError(pub CoreError);

impl From<CoreError> for ApiError {
    fn from(value: CoreError) -> Self {
        Self(value)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self(CoreError::msg(value.to_string()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self.0 {
            CoreError::Unauthorized => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::Forbidden => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::NotFound => (StatusCode::NOT_FOUND, self.0.to_string()),
            CoreError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            CoreError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            CoreError::VaultLocked => (StatusCode::LOCKED, self.0.to_string()),
            CoreError::VaultUnlockFailed => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::Msg(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            CoreError::Other(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
