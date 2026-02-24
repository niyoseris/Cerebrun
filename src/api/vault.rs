use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::auth::{AuthenticatedAgent, SessionUser};
use crate::crypto::hash::{generate_vault_token, sha256_hash};
use crate::crypto::vault as vault_crypto;
use crate::db;
use crate::error::AppError;
use crate::models::{VaultAccessRequest, VaultApproveRequest, VaultDenyRequest};
use crate::AppState;

pub async fn request_vault_access(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(req): Json<VaultAccessRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let consent = db::vault::create_consent_request(
        &state.pool,
        agent.api_key.user_id,
        agent.api_key.id,
        &req.reason,
        &serde_json::to_value(&req.requested_fields).unwrap(),
    )
    .await?;

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "vault_access_request", Some("3"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "request_id": consent.id,
        "status": "pending",
    })))
}

pub async fn approve_vault_request(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<VaultApproveRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let vault_token = generate_vault_token();
    let token_hash = sha256_hash(&vault_token);
    let expires_in = req.expires_in_hours.unwrap_or(24);

    let approved = db::vault::approve_consent_request(
        &state.pool,
        req.request_id,
        session.user.id,
        &token_hash,
        expires_in,
    )
    .await?;

    if !approved {
        return Err(AppError::NotFound("Consent request not found or already resolved".to_string()));
    }

    let _ = db::audit::log_access(
        &state.pool, session.user.id, None,
        "vault_consent_approved", Some("3"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "vault_token": vault_token,
        "expires_in_hours": expires_in,
    })))
}

pub async fn deny_vault_request(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<VaultDenyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let denied = db::vault::deny_consent_request(
        &state.pool,
        req.request_id,
        session.user.id,
    )
    .await?;

    if !denied {
        return Err(AppError::NotFound("Consent request not found or already resolved".to_string()));
    }

    let _ = db::audit::log_access(
        &state.pool, session.user.id, None,
        "vault_consent_denied", Some("3"), true, None, None, None,
    ).await;

    Ok(Json(json!({"status": "ok"})))
}

pub async fn get_vault_context(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let vault_token = headers
        .get("X-Vault-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing X-Vault-Token header".to_string()))?;

    // Special case for dashboard to see key names (not values)
    if vault_token == "metadata-only" {
        let vault_data = db::vault::get_vault(&state.pool, agent.api_key.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No vault data found".to_string()))?;

        let key = vault_crypto::derive_vault_key(&state.config.session_secret);
        let decrypted = vault_crypto::decrypt_vault_data(&vault_data.encrypted_data, &key)
            .map_err(|e| AppError::Internal(format!("Vault decryption failed: {}", e)))?;

        let all_data: serde_json::Value = serde_json::from_slice(&decrypted)
            .map_err(|e| AppError::Internal(format!("Invalid vault data: {}", e)))?;
        
        let keys: Vec<String> = all_data.as_object().map(|obj| obj.keys().cloned().collect()).unwrap_or_default();
        return Ok(Json(json!({ "keys": keys })));
    }

    let token_hash = sha256_hash(vault_token);

    let consent = db::vault::validate_vault_token(&state.pool, &token_hash, agent.api_key.id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Invalid or expired vault token".to_string()))?;

    let vault_data = db::vault::get_vault(&state.pool, agent.api_key.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("No vault data found".to_string()))?;

    let key = vault_crypto::derive_vault_key(&state.config.session_secret);
    let decrypted = vault_crypto::decrypt_vault_data(&vault_data.encrypted_data, &key)
        .map_err(|e| AppError::Internal(format!("Vault decryption failed: {}", e)))?;

    let all_data: serde_json::Value = serde_json::from_slice(&decrypted)
        .map_err(|e| AppError::Internal(format!("Invalid vault data: {}", e)))?;

    let requested_fields: Vec<String> = serde_json::from_value(consent.requested_data)
        .unwrap_or_default();

    let mut filtered = serde_json::Map::new();
    for field in &requested_fields {
        let parts: Vec<&str> = field.split('.').collect();
        if let Some(value) = all_data.get(parts[0]) {
            if parts.len() > 1 {
                if let Some(inner) = value.get(parts[1]) {
                    filtered.insert(field.clone(), inner.clone());
                }
            } else {
                filtered.insert(field.clone(), value.clone());
            }
        }
    }

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "vault_access", Some("3"), true, None, None, None,
    ).await;

    Ok(Json(serde_json::Value::Object(filtered)))
}

pub async fn put_vault_context(
    State(state): State<AppState>,
    session: SessionUser,
    Json(new_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut current_data = if let Some(vault) = db::vault::get_vault(&state.pool, session.user.id).await? {
        let key = vault_crypto::derive_vault_key(&state.config.session_secret);
        let decrypted = vault_crypto::decrypt_vault_data(&vault.encrypted_data, &key)
            .map_err(|e| AppError::Internal(format!("Vault decryption failed: {}", e)))?;
        serde_json::from_slice(&decrypted).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if let Some(obj) = new_data.as_object() {
        if let Some(current_obj) = current_data.as_object_mut() {
            for (k, v) in obj {
                current_obj.insert(k.clone(), v.clone());
            }
        }
    }

    let key = vault_crypto::derive_vault_key(&state.config.session_secret);
    let encrypted = vault_crypto::encrypt_vault_data(&serde_json::to_vec(&current_data).unwrap(), &key)
        .map_err(|e| AppError::Internal(format!("Vault encryption failed: {}", e)))?;
    
    let key_hash = sha256_hash(&state.config.session_secret);

    db::vault::upsert_vault(&state.pool, session.user.id, &key_hash, &encrypted).await?;

    let _ = db::audit::log_access(
        &state.pool, session.user.id, None,
        "update_vault", Some("3"), true, None, None, None,
    ).await;

    Ok(Json(json!({"status": "ok"})))
}

pub async fn get_pending_consents(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let requests = db::vault::get_pending_consent_requests(&state.pool, session.user.id).await?;
    Ok(Json(serde_json::to_value(requests).unwrap()))
}
