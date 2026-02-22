use axum::extract::State;
use axum::Json;
use serde_json::json;
use crate::auth::SessionUser;
use crate::db;
use crate::error::AppError;
use crate::models::system::*;
use crate::AppState;

pub async fn list_system_models(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<Vec<SystemModel>>, AppError> {
    if !session.user.is_admin {
        return Err(AppError::Forbidden("Admin only".to_string()));
    }
    let models = db::system::list_system_models(&state.pool).await?;
    Ok(Json(models))
}

pub async fn add_system_model(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<AddSystemModelRequest>,
) -> Result<Json<SystemModel>, AppError> {
    if !session.user.is_admin {
        return Err(AppError::Forbidden("Admin only".to_string()));
    }
    let model = db::system::add_system_model(&state.pool, &req.provider, &req.model_name, req.display_name.as_deref()).await?;
    Ok(Json(model))
}

pub async fn delete_system_model(
    State(state): State<AppState>,
    session: SessionUser,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !session.user.is_admin {
        return Err(AppError::Forbidden("Admin only".to_string()));
    }
    db::system::delete_system_model(&state.pool, id).await?;
    Ok(Json(json!({"status": "ok"})))
}

pub async fn get_settings(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<serde_json::Value>, AppError> {
    if !session.user.is_admin {
        return Err(AppError::Forbidden("Admin only".to_string()));
    }
    let auto_embed = db::system::get_setting(&state.pool, "auto_embedding").await?.unwrap_or(json!(true));
    let top_k = db::system::get_setting(&state.pool, "vector_top_k").await?.unwrap_or(json!(5));
    let min_score = db::system::get_setting(&state.pool, "vector_min_score").await?.unwrap_or(json!(0.7));
    let embed_provider = db::system::get_setting(&state.pool, "embedding_provider").await?.unwrap_or(json!("openai"));
    
    Ok(Json(json!({ 
        "auto_embedding": auto_embed,
        "vector_top_k": top_k,
        "vector_min_score": min_score,
        "embedding_provider": embed_provider
    })))
}

pub async fn update_setting(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !session.user.is_admin {
        return Err(AppError::Forbidden("Admin only".to_string()));
    }
    if let Some(key) = req.get("key").and_then(|v| v.as_str()) {
        if let Some(value) = req.get("value") {
            db::system::set_setting(&state.pool, key, value.clone()).await?;
        }
    }
    Ok(Json(json!({"status": "ok"})))
}
