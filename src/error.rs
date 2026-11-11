use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("OAuth error: {0}")]
    OAuth(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "Forbidden"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "Not found"),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            AppError::OAuth(_) => (StatusCode::BAD_REQUEST, "OAuth error"),
        };

        // Log the full details server-side, but never leak database errors,
        // internal errors, or OAuth provider responses to the client.
        match &self {
            AppError::BadRequest(_) => {}
            AppError::OAuth(_) => tracing::warn!("OAuth error: {}", self),
            AppError::Unauthorized(_) | AppError::Forbidden(_) | AppError::NotFound(_) => {
                tracing::debug!("Request rejected: {}", self)
            }
            AppError::Database(_) | AppError::Internal(_) => {
                tracing::error!("Request failed: {}", self)
            }
        }

        let body = match &self {
            AppError::BadRequest(_) => json!({
                "error": message,
                "details": self.to_string()
            }),
            _ => json!({ "error": message }),
        };

        (status, axum::Json(body)).into_response()
    }
}
