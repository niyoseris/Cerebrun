use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde_json::Value;
use uuid::Uuid;

use crate::crypto::hash::sha256_hash;
use crate::db;
use crate::error::AppError;
use crate::models::User;
use crate::AppState;

/// Authentication context that accepts either:
///  * an agent API key in `Authorization: Bearer <key>`, or
///  * a browser session cookie created by the Google OAuth dashboard login.
///
/// Dashboard requests authenticate with the session cookie, while MCP / API
/// clients authenticate with API keys. Layer endpoints are shared by both, so
/// they need this combined extractor instead of the API-key-only one.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: User,
    pub user_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub permissions: Option<Value>,
    pub is_session: bool,
}

impl AuthContext {
    pub fn has_permission(&self, layer: &str) -> bool {
        // The user themself, signed in through the dashboard, has full access
        // to their own context.
        if self.is_session {
            return true;
        }

        self.permissions
            .as_ref()
            .and_then(|p| p.get(layer))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthContext {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // API key takes precedence when an Authorization header is present, so
        // a bad API key never silently falls back to a browser session.
        if let Some(auth_header) = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        {
            let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
                AppError::Unauthorized("Invalid Authorization format".to_string())
            })?;

            let key_hash = sha256_hash(token);
            let api_key = db::api_keys::get_api_key_by_hash(&state.pool, &key_hash)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;

            let _ = db::api_keys::update_last_used(&state.pool, api_key.id).await;

            let user = db::users::get_user_by_id(&state.pool, api_key.user_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or_else(|| AppError::Unauthorized("API key owner not found".to_string()))?;

            return Ok(AuthContext {
                user_id: user.id,
                user,
                api_key_id: Some(api_key.id),
                permissions: Some(api_key.permissions),
                is_session: false,
            });
        }

        let cookie_header = parts
            .headers
            .get(http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::Unauthorized("Missing Authorization header or session cookie".to_string())
            })?;

        let session_token = cookie_header
            .split(';')
            .filter_map(|s| s.trim().strip_prefix("session="))
            .next()
            .ok_or_else(|| AppError::Unauthorized("No session token".to_string()))?;

        let token_hash = sha256_hash(session_token);

        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT user_id FROM sessions WHERE token_hash = $1 AND expires_at > NOW()",
        )
        .bind(&token_hash)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired session".to_string()))?;

        let user = db::users::get_user_by_id(&state.pool, row.0)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("Session user not found".to_string()))?;

        Ok(AuthContext {
            user_id: user.id,
            user,
            api_key_id: None,
            permissions: None,
            is_session: true,
        })
    }
}
