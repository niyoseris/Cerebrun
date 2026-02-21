use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::db;
use crate::error::AppError;
use crate::models::KnowledgeEntryInfo;
use crate::AppState;

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
