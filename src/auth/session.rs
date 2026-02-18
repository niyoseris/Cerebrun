use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::crypto::hash::{generate_session_token, sha256_hash};
use crate::error::AppError;
use crate::models::User;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub user: User,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for SessionUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let pool = state.pool.clone();

        let cookie_header = parts.headers
            .get(http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("No session cookie".to_string()))?;

        let session_token = cookie_header
            .split(';')
            .filter_map(|s| {
                let s = s.trim();
                s.strip_prefix("session=")
            })
            .next()
            .ok_or_else(|| AppError::Unauthorized("No session token".to_string()))?;

        let token_hash = sha256_hash(session_token);

        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT user_id FROM sessions WHERE token_hash = $1 AND expires_at > NOW()",
        )
        .bind(&token_hash)
        .fetch_optional(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired session".to_string()))?;

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(row.0)
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(SessionUser { user })
    }
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    let token = generate_session_token();
    let token_hash = sha256_hash(&token);
    let expires_at = Utc::now() + Duration::days(7);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}
