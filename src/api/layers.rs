use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::auth::{AuthenticatedAgent, SessionUser};
use crate::db;
use crate::error::AppError;
use crate::models::{Layer0Update, Layer1Update, Layer2Update};
use crate::AppState;

pub async fn get_layer0(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
) -> Result<Json<serde_json::Value>, AppError> {
    let data = db::layers::get_layer0(&state.pool, agent.api_key.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Layer 0 data not found".to_string()))?;

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "read_layer0", Some("0"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "language": data.language,
        "timezone": data.timezone,
        "output_format_preferences": data.output_format,
        "blocked_topics": data.blocked_topics,
        "communication_style": data.communication_style,
    })))
}

pub async fn put_layer0(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(data): Json<Layer0Update>,
) -> Result<Json<serde_json::Value>, AppError> {
    let updated = db::layers::update_layer0(&state.pool, agent.api_key.user_id, &data).await?;

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "update_layer0", Some("0"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "language": updated.language,
        "timezone": updated.timezone,
        "output_format_preferences": updated.output_format,
        "blocked_topics": updated.blocked_topics,
        "communication_style": updated.communication_style,
    })))
}

pub async fn get_layer1(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
) -> Result<Json<serde_json::Value>, AppError> {
    if !agent.has_permission("layer1") {
        let _ = db::audit::log_access(
            &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
            "read_layer1", Some("1"), false, None, None, None,
        ).await;
        return Err(AppError::Forbidden("No permission for Layer 1".to_string()));
    }

    let data = db::layers::get_layer1(&state.pool, agent.api_key.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Layer 1 data not found".to_string()))?;

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "read_layer1", Some("1"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "active_projects": data.active_projects,
        "recent_conversations": data.recent_conversations,
        "working_directories": data.working_directories,
        "current_goals": data.current_goals,
        "pinned_memories": data.pinned_memories,
    })))
}

pub async fn put_layer1(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(data): Json<Layer1Update>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !agent.has_permission("layer1") {
        return Err(AppError::Forbidden("No permission for Layer 1".to_string()));
    }

    let updated = db::layers::update_layer1(&state.pool, agent.api_key.user_id, &data).await?;

    // Auto-embed Layer 1 context
    if let Some(llm_key) = db::llm_keys::get_any_embedding_key(&state.pool, agent.api_key.user_id).await.ok().flatten() {
        let mut context_text = String::new();
        if let Some(projects) = &updated.active_projects {
            if let Some(arr) = projects.as_array() {
                let p_list: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                context_text.push_str(&format!("Projects: {}\n", p_list.join(", ")));
            }
        }
        if let Some(goals) = &updated.current_goals {
            if let Some(arr) = goals.as_array() {
                let g_list: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                context_text.push_str(&format!("Goals: {}\n", g_list.join("\n")));
            }
        }
        if let Some(memories) = &updated.pinned_memories {
            if let Some(arr) = memories.as_array() {
                let m_list: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                context_text.push_str(&format!("Memories: {}\n", m_list.join("\n")));
            }
        }
        
        if !context_text.is_empty() {
            let api_key = crate::api::llm::decrypt_provider_key(&state, &llm_key.encrypted_key).ok();
            if let Some(key) = api_key {
                if let Ok(resp) = crate::llm::provider::get_embedding(&llm_key.provider, &key, &context_text).await {
                    let _ = db::embeddings::upsert_context_embedding(&state.pool, agent.api_key.user_id, "layer1", "all", &context_text, &resp.embedding).await;
                }
            }
        }
    }

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "update_layer1", Some("1"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "active_projects": updated.active_projects,
        "recent_conversations": updated.recent_conversations,
        "working_directories": updated.working_directories,
        "current_goals": updated.current_goals,
        "pinned_memories": updated.pinned_memories,
    })))
}

pub async fn get_layer2(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
) -> Result<Json<serde_json::Value>, AppError> {
    if !agent.has_permission("layer2") {
        let _ = db::audit::log_access(
            &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
            "read_layer2", Some("2"), false, None, None, None,
        ).await;
        return Err(AppError::Forbidden("No permission for Layer 2".to_string()));
    }

    let data = db::layers::get_layer2(&state.pool, agent.api_key.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Layer 2 data not found".to_string()))?;

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "read_layer2", Some("2"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "display_name": data.display_name,
        "location": data.location,
        "interests": data.interests,
        "contact_preferences": data.contact_preferences,
        "relationship_notes": data.relationship_notes,
    })))
}

pub async fn put_layer2(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(data): Json<Layer2Update>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !agent.has_permission("layer2") {
        return Err(AppError::Forbidden("No permission for Layer 2".to_string()));
    }

    let updated = db::layers::update_layer2(&state.pool, agent.api_key.user_id, &data).await?;

    let _ = db::audit::log_access(
        &state.pool, agent.api_key.user_id, Some(agent.api_key.id),
        "update_layer2", Some("2"), true, None, None, None,
    ).await;

    Ok(Json(json!({
        "display_name": updated.display_name,
        "location": updated.location,
        "interests": updated.interests,
        "contact_preferences": updated.contact_preferences,
        "relationship_notes": updated.relationship_notes,
    })))
}

pub async fn get_me(session: SessionUser) -> Json<serde_json::Value> {
    let user = session.user;
    Json(json!({
        "id": user.id,
        "email": user.email,
        "display_name": user.display_name,
        "avatar_url": user.avatar_url,
    }))
}
