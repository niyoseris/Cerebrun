use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::db;
use crate::error::AppError;
use crate::models::{KnowledgeEntryInfo, PushKnowledgeRequest};
use crate::AppState;
use crate::llm::provider;

#[derive(Debug, Deserialize)]
pub struct KnowledgeQueryParams {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub source_project: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_knowledge(
    State(state): State<AppState>,
    session: SessionUser,
    Query(params): Query<KnowledgeQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    let entries = db::knowledge::query_knowledge(
        &state.pool,
        session.user.id,
        params.keyword.as_deref(),
        params.category.as_deref(),
        params.tag.as_deref(),
        params.source_project.as_deref(),
        limit,
        offset,
    )
    .await?;

    let total = db::knowledge::count_knowledge(
        &state.pool,
        session.user.id,
        params.keyword.as_deref(),
        params.category.as_deref(),
        params.tag.as_deref(),
        params.source_project.as_deref(),
    ).await?;
    let categories = db::knowledge::list_categories(&state.pool, session.user.id).await?;

    let items: Vec<KnowledgeEntryInfo> = entries.into_iter().map(KnowledgeEntryInfo::from).collect();

    Ok(Json(json!({
        "entries": items,
        "total": total,
        "categories": categories,
        "limit": limit,
        "offset": offset,
    })))
}

pub async fn get_knowledge(
    State(state): State<AppState>,
    session: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let entry = db::knowledge::get_knowledge_by_id(&state.pool, id, session.user.id)
        .await?
        .ok_or(AppError::NotFound("Knowledge entry not found".to_string()))?;

    let info: KnowledgeEntryInfo = entry.into();
    Ok(Json(json!(info)))
}

pub async fn create_knowledge(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<PushKnowledgeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = session.user.id;
    let category = req.category.as_deref().unwrap_or("uncategorized");
    
    let entry = db::knowledge::insert_knowledge(
        &state.pool,
        user_id,
        &req.content,
        req.summary.as_deref(),
        category,
        req.subcategory.as_deref(),
        req.tags.as_ref().unwrap_or(&vec![]),
        req.source_agent.as_deref(),
        req.source_project.as_deref(),
        None,
    ).await?;

    // Auto-embedding
    if db::system::is_auto_embedding_enabled(&state.pool).await {
        let target_provider = db::system::get_embedding_provider(&state.pool).await;
        if let Ok(Some(llm_key)) = db::llm_keys::get_provider_key(&state.pool, user_id, &target_provider).await {
             let vault_key = crate::crypto::vault_crypto::derive_vault_key(&state.config.session_secret);
             if let Ok(decrypted) = crate::crypto::vault_crypto::decrypt_vault_data(&llm_key.encrypted_key, &vault_key) {
                if let Ok(api_key) = String::from_utf8(decrypted) {
                    let embed_text = if let Some(s) = &req.summary {
                        format!("{}: {}", s, req.content)
                    } else {
                        req.content.clone()
                    };
                    
                    if let Ok(resp) = provider::get_embedding(&target_provider, &api_key, &embed_text).await {
                        let _ = db::embeddings::update_knowledge_embedding(
                            &state.pool, entry.id, &resp.embedding,
                        ).await;
                    }
                }
             }
        }
    }

    Ok(Json(json!(KnowledgeEntryInfo::from(entry))))
}

pub async fn delete_knowledge(
    State(state): State<AppState>,
    session: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::knowledge::delete_knowledge(&state.pool, id, session.user.id).await?;

    if deleted {
        Ok(Json(json!({"status": "deleted"})))
    } else {
        Err(AppError::NotFound("Knowledge entry not found".to_string()))
    }
}
