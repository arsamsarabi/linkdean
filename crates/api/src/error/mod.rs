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
        if let CoreError::Banned { reason, ends_at } = &self.0 {
            let body = json!({
                "error": "banned",
                "code": "banned",
                "ban": { "reason": reason, "ends_at": ends_at },
            });
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        let (status, msg) = match &self.0 {
            CoreError::Banned { .. } => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::Unauthorized => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::Forbidden => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::NotFound => (StatusCode::NOT_FOUND, self.0.to_string()),
            CoreError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            CoreError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            CoreError::VaultLocked => (StatusCode::LOCKED, self.0.to_string()),
            CoreError::VaultUnlockFailed => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::Msg(m) => {
                tracing::error!(error = %m, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
            CoreError::Other(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
