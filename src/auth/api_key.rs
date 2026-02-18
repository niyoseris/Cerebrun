use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::crypto::hash::sha256_hash;
use crate::db;
use crate::error::AppError;
use crate::models::ApiKey;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedAgent {
    pub api_key: ApiKey,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthenticatedAgent {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let pool = state.pool.clone();

        let auth_header = parts.headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid Authorization format".to_string()))?;

        let key_hash = sha256_hash(token);

        let api_key = db::api_keys::get_api_key_by_hash(&pool, &key_hash)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;

        let _ = db::api_keys::update_last_used(&pool, api_key.id).await;

        Ok(AuthenticatedAgent { api_key })
    }
}

impl AuthenticatedAgent {
    pub fn has_permission(&self, layer: &str) -> bool {
        self.api_key
            .permissions
            .get(layer)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}
