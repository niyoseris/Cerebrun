use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::SessionUser;
use crate::db;
use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub layer: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_audit_log(
    State(state): State<AppState>,
    session: SessionUser,
    Query(query): Query<AuditQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let entries = db::audit::get_audit_log(
        &state.pool,
        session.user.id,
        query.layer.as_deref(),
        limit,
        offset,
    )
    .await?;

    Ok(Json(json!({
        "entries": entries,
        "limit": limit,
        "offset": offset,
    })))
}

pub async fn export_data(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = session.user.id;

    let layer0 = db::layers::get_layer0(&state.pool, user_id).await?;
    let layer1 = db::layers::get_layer1(&state.pool, user_id).await?;
    let layer2 = db::layers::get_layer2(&state.pool, user_id).await?;
    let keys = db::api_keys::list_api_keys(&state.pool, user_id).await?;

    let key_infos: Vec<crate::models::ApiKeyInfo> = keys
        .into_iter()
        .map(crate::models::ApiKeyInfo::from)
        .collect();

    // Data portability: include the knowledge base and the full LLM
    // conversation history, not just the three context layers. Vault secrets
    // are intentionally excluded from the export for safety.
    let knowledge =
        db::knowledge::query_knowledge(&state.pool, user_id, None, None, None, None, 10_000, 0)
            .await?;

    let conversations = db::conversations::list_conversations(&state.pool, user_id).await?;
    let mut conversation_exports = Vec::new();
    for conv in conversations {
        let messages = db::conversations::get_messages(&state.pool, conv.id).await?;
        conversation_exports.push(json!({
            "conversation": conv,
            "messages": messages,
        }));
    }

    Ok(Json(json!({
        "user": {
            "id": session.user.id,
            "email": session.user.email,
            "display_name": session.user.display_name,
        },
        "layer0": layer0,
        "layer1": layer1,
        "layer2": layer2,
        "api_keys": key_infos,
        "knowledge": knowledge,
        "conversations": conversation_exports,
        "vault": null,
    })))
}

#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub confirm: String,
}

pub async fn delete_account(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<DeleteAccountRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.confirm != "DELETE" {
        return Err(AppError::BadRequest(
            "Must confirm with 'DELETE'".to_string(),
        ));
    }

    db::users::delete_user(&state.pool, session.user.id).await?;

    Ok(Json(json!({"status": "ok", "message": "Account deleted"})))
}
