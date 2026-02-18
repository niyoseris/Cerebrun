use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::crypto::hash::{generate_api_key, sha256_hash};
use crate::db;
use crate::error::AppError;
use crate::models::{ApiKeyInfo, CreateApiKeyRequest, CreateApiKeyResponse};
use crate::AppState;

pub async fn list_keys(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<Vec<ApiKeyInfo>>, AppError> {
    let keys = db::api_keys::list_api_keys(&state.pool, session.user.id).await?;
    let infos: Vec<ApiKeyInfo> = keys.into_iter().map(ApiKeyInfo::from).collect();
    Ok(Json(infos))
}

pub async fn create_key(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    let raw_key = generate_api_key();
    let key_hash = sha256_hash(&raw_key);

    let api_key = db::api_keys::create_api_key(
        &state.pool,
        session.user.id,
        &req.name,
        &key_hash,
        &req.permissions,
    )
    .await?;

    Ok(Json(CreateApiKeyResponse {
        id: api_key.id,
        key: raw_key,
    }))
}

pub async fn delete_key(
    State(state): State<AppState>,
    session: SessionUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::api_keys::delete_api_key(&state.pool, key_id, session.user.id).await?;
    if !deleted {
        return Err(AppError::NotFound("API key not found".to_string()));
    }
    Ok(Json(json!({"status": "ok"})))
}

pub async fn revoke_key(
    State(state): State<AppState>,
    session: SessionUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let revoked = db::api_keys::revoke_api_key(&state.pool, key_id, session.user.id).await?;
    if !revoked {
        return Err(AppError::NotFound("API key not found".to_string()));
    }
    Ok(Json(json!({"status": "ok"})))
}
