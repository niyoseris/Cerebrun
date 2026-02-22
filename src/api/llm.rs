use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures::stream::Stream;
use serde_json::json;
use std::convert::Infallible;
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::crypto::vault as vault_crypto;
use crate::db;
use crate::error::AppError;
use crate::llm::provider;
use crate::models::*;
use crate::AppState;

pub async fn add_provider_key(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<AddProviderKeyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let valid = provider::validate_key(&req.provider, &req.api_key)
        .await
        .map_err(|e| AppError::Internal(format!("Key validation failed: {}", e)))?;

    if !valid {
        return Err(AppError::BadRequest("Invalid API key. Please check and try again.".to_string()));
    }

    let key = vault_crypto::derive_vault_key(&state.config.session_secret);
    let encrypted = vault_crypto::encrypt_vault_data(req.api_key.as_bytes(), &key)
        .map_err(|e| AppError::Internal(e))?;

    let record = db::llm_keys::add_provider_key(
        &state.pool,
        session.user.id,
        &req.provider,
        &req.key_name,
        &encrypted,
    )
    .await?;

    Ok(Json(json!({
        "id": record.id,
        "provider": record.provider,
        "key_name": record.key_name,
        "status": "active",
        "validated": true,
    })))
}

pub async fn list_provider_keys(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<Vec<LlmProviderKeyInfo>>, AppError> {
    let keys = db::llm_keys::list_provider_keys(&state.pool, session.user.id).await?;
    Ok(Json(keys.into_iter().map(LlmProviderKeyInfo::from).collect()))
}

pub async fn delete_provider_key(
    State(state): State<AppState>,
    session: SessionUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::llm_keys::delete_provider_key(&state.pool, key_id, session.user.id).await?;
    if !deleted {
        return Err(AppError::NotFound("Provider key not found".to_string()));
    }
    Ok(Json(json!({"status": "ok"})))
}

pub fn decrypt_provider_key(state: &AppState, encrypted: &[u8]) -> Result<String, AppError> {
    let key = vault_crypto::derive_vault_key(&state.config.session_secret);
    let decrypted = vault_crypto::decrypt_vault_data(encrypted, &key)
        .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;
    String::from_utf8(decrypted).map_err(|e| AppError::Internal(e.to_string()))
}

fn build_system_prompt_layer0_only(layer0: Option<&Layer0Public>) -> String {
    let mut parts = Vec::new();

    if let Some(l0) = layer0 {
        let mut prefs = Vec::new();
        if let Some(lang) = &l0.language {
            prefs.push(format!("Language: {}", lang));
        }
        if let Some(tz) = &l0.timezone {
            prefs.push(format!("Timezone: {}", tz));
        }
        if let Some(style) = &l0.communication_style {
            prefs.push(format!("Communication style: {}", style));
        }
        if !prefs.is_empty() {
            parts.push(format!("[User Preferences]\n{}", prefs.join("\n")));
        }
    }

    if parts.is_empty() {
        return String::new();
    }

    format!("You are a helpful assistant. Here is context about the user:\n\n{}", parts.join("\n\n"))
}

pub async fn create_conversation(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<Conversation>, AppError> {
    let inject = req.inject_context.unwrap_or(true);
    let budget = req.context_token_budget.unwrap_or(2000);

    let system_prompt = if inject {
        let layer0 = db::layers::get_layer0(&state.pool, session.user.id).await?;
        build_system_prompt_layer0_only(layer0.as_ref())
    } else {
        String::new()
    };

    let conv = db::conversations::create_conversation(
        &state.pool,
        session.user.id,
        &req.provider,
        &req.model,
        req.title.as_deref(),
        if system_prompt.is_empty() { None } else { Some(&system_prompt) },
        None,
        None,
    )
    .await?;

    if inject != true || budget != 2000 {
        let _ = sqlx::query(
            "UPDATE conversations SET inject_context = $1, context_token_budget = $2 WHERE id = $3"
        )
        .bind(inject)
        .bind(budget)
        .execute(&state.pool)
        .await;
    }

    Ok(Json(conv))
}

pub async fn list_conversations(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<Vec<Conversation>>, AppError> {
    let convs = db::conversations::list_conversations(&state.pool, session.user.id).await?;
    Ok(Json(convs))
}

pub async fn get_conversation_messages(
    State(state): State<AppState>,
    session: SessionUser,
    Path(conv_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conv = db::conversations::get_conversation(&state.pool, conv_id, session.user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;

    let messages = db::conversations::get_messages(&state.pool, conv_id).await?;

    Ok(Json(json!({
        "conversation": conv,
        "messages": messages,
    })))
}

pub async fn delete_conversation(
    State(state): State<AppState>,
    session: SessionUser,
    Path(conv_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::conversations::delete_conversation(&state.pool, conv_id, session.user.id).await?;
    if !deleted {
        return Err(AppError::NotFound("Conversation not found".to_string()));
    }
    Ok(Json(json!({"status": "ok"})))
}

pub async fn chat(
    State(state): State<AppState>,
    session: SessionUser,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AppError> {
    let conv = db::conversations::get_conversation(&state.pool, conv_id, session.user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;

    let prov = req.provider.as_deref().unwrap_or(&conv.provider);
    let mdl = req.model.as_deref().unwrap_or(&conv.model);

    let provider_key = db::llm_keys::get_provider_key(&state.pool, session.user.id, prov)
        .await?
        .ok_or_else(|| AppError::BadRequest(format!("No API key configured for provider: {}", prov)))?;

    let api_key = decrypt_provider_key(&state, &provider_key.encrypted_key)?;

    let _user_msg = db::conversations::add_message(
        &state.pool, conv_id, "user", &req.message, None, None, 0, 0, 0,
    ).await?;

    let history = db::conversations::get_messages(&state.pool, conv_id).await?;
    let mut llm_messages: Vec<provider::LlmMessage> = Vec::new();

    if let Some(sys) = &conv.system_prompt {
        if !sys.is_empty() {
            llm_messages.push(provider::LlmMessage {
                role: "system".to_string(),
                content: sys.clone(),
            });
        }
    }

    for msg in &history {
        llm_messages.push(provider::LlmMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    let response = provider::call_llm(prov, mdl, &api_key, &llm_messages)
        .await
        .map_err(|e| AppError::Internal(e))?;

    let assistant_msg = db::conversations::add_message(
        &state.pool, conv_id, "assistant", &response.content,
        Some(prov), Some(mdl),
        response.prompt_tokens, response.completion_tokens, response.total_tokens,
    ).await?;

    let _ = db::llm_usage::record_usage(
        &state.pool, session.user.id, Some(conv_id), Some(assistant_msg.id),
        prov, mdl, response.prompt_tokens, response.completion_tokens,
        response.total_tokens,
    ).await;

    Ok(Json(ChatResponse {
        message: assistant_msg,
        usage: TokenUsage {
            prompt_tokens: response.prompt_tokens,
            completion_tokens: response.completion_tokens,
            total_tokens: response.total_tokens,
        },
    }))
}

pub async fn compare(
    State(state): State<AppState>,
    session: SessionUser,
    Json(req): Json<CompareRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let layer0 = db::layers::get_layer0(&state.pool, session.user.id).await?;
    let system_prompt = build_system_prompt_layer0_only(layer0.as_ref());

    let mut results = Vec::new();

    for target in &req.targets {
        let provider_key = db::llm_keys::get_provider_key(&state.pool, session.user.id, &target.provider)
            .await?
            .ok_or_else(|| AppError::BadRequest(format!("No API key for: {}", target.provider)))?;

        let api_key = decrypt_provider_key(&state, &provider_key.encrypted_key)?;

        let mut messages = Vec::new();
        if !system_prompt.is_empty() {
            messages.push(provider::LlmMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }
        messages.push(provider::LlmMessage {
            role: "user".to_string(),
            content: req.message.clone(),
        });

        let start = std::time::Instant::now();
        let response = provider::call_llm(&target.provider, &target.model, &api_key, &messages).await;
        let elapsed = start.elapsed();

        match response {
            Ok(resp) => {
                let _ = db::llm_usage::record_usage(
                    &state.pool, session.user.id, None, None,
                    &target.provider, &target.model,
                    resp.prompt_tokens, resp.completion_tokens, resp.total_tokens,
                ).await;

                results.push(json!({
                    "provider": target.provider,
                    "model": target.model,
                    "content": resp.content,
                    "usage": {
                        "prompt_tokens": resp.prompt_tokens,
                        "completion_tokens": resp.completion_tokens,
                        "total_tokens": resp.total_tokens,
                    },
                    "latency_ms": elapsed.as_millis(),
                    "status": "success",
                }));
            }
            Err(e) => {
                results.push(json!({
                    "provider": target.provider,
                    "model": target.model,
                    "content": null,
                    "error": e,
                    "latency_ms": elapsed.as_millis(),
                    "status": "error",
                }));
            }
        }
    }

    Ok(Json(json!({ "results": results })))
}

pub async fn fork_conversation(
    State(state): State<AppState>,
    session: SessionUser,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<ForkRequest>,
) -> Result<Json<Conversation>, AppError> {
    let _original = db::conversations::get_conversation(&state.pool, conv_id, session.user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;

    let messages_to_copy = db::conversations::get_messages_up_to(&state.pool, conv_id, req.message_id).await?;

    let layer0 = db::layers::get_layer0(&state.pool, session.user.id).await?;
    let system_prompt = build_system_prompt_layer0_only(layer0.as_ref());

    let new_conv = db::conversations::create_conversation(
        &state.pool,
        session.user.id,
        &req.new_provider,
        &req.new_model,
        Some(&format!("Fork from {} to {}", _original.model, req.new_model)),
        if system_prompt.is_empty() { None } else { Some(&system_prompt) },
        Some(conv_id),
        Some(req.message_id),
    )
    .await?;

    for msg in &messages_to_copy {
        let _ = db::conversations::add_message(
            &state.pool, new_conv.id, &msg.role, &msg.content,
            msg.provider.as_deref(), msg.model.as_deref(),
            msg.prompt_tokens.unwrap_or(0), msg.completion_tokens.unwrap_or(0),
            msg.total_tokens.unwrap_or(0),
        ).await?;
    }

    Ok(Json(new_conv))
}

pub async fn get_usage_metrics(
    State(state): State<AppState>,
    session: SessionUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let summary = db::llm_usage::get_usage_summary(&state.pool, session.user.id).await?;

    let total_tokens: i64 = summary.iter().map(|s| s.total_tokens.unwrap_or(0)).sum();

    Ok(Json(json!({
        "total_tokens": total_tokens,
        "by_provider": summary,
    })))
}

pub async fn stream_chat(
    State(state): State<AppState>,
    session: SessionUser,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let conv = db::conversations::get_conversation(&state.pool, conv_id, session.user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;

    let prov = req.provider.as_deref().unwrap_or(&conv.provider).to_string();
    let mdl = req.model.as_deref().unwrap_or(&conv.model).to_string();

    let provider_key = db::llm_keys::get_provider_key(&state.pool, session.user.id, &prov)
        .await?
        .ok_or_else(|| AppError::BadRequest(format!("No API key for: {}", prov)))?;

    let api_key = decrypt_provider_key(&state, &provider_key.encrypted_key)?;

    let _user_msg = db::conversations::add_message(
        &state.pool, conv_id, "user", &req.message, None, None, 0, 0, 0,
    ).await?;

    let history = db::conversations::get_messages(&state.pool, conv_id).await?;
    let mut llm_messages: Vec<provider::LlmMessage> = Vec::new();

    if let Some(sys) = &conv.system_prompt {
        if !sys.is_empty() {
            llm_messages.push(provider::LlmMessage {
                role: "system".to_string(),
                content: sys.clone(),
            });
        }
    }

    for msg in &history {
        llm_messages.push(provider::LlmMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    let pool = state.pool.clone();
    let user_id = session.user.id;

    let stream = async_stream::stream! {
        yield Ok(Event::default().data(json!({"type": "start", "provider": &prov, "model": &mdl}).to_string()));

        match provider::call_llm(&prov, &mdl, &api_key, &llm_messages).await {
            Ok(response) => {
                for chunk in response.content.chars().collect::<Vec<_>>().chunks(10) {
                    let text: String = chunk.iter().collect();
                    yield Ok(Event::default().data(json!({"type": "chunk", "text": text}).to_string()));
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }

                let assistant_msg = db::conversations::add_message(
                    &pool, conv_id, "assistant", &response.content,
                    Some(&prov), Some(&mdl),
                    response.prompt_tokens, response.completion_tokens, response.total_tokens,
                ).await;

                if let Ok(msg) = &assistant_msg {
                    let _ = db::llm_usage::record_usage(
                        &pool, user_id, Some(conv_id), Some(msg.id),
                        &prov, &mdl, response.prompt_tokens, response.completion_tokens,
                        response.total_tokens,
                    ).await;
                }

                yield Ok(Event::default().data(json!({
                    "type": "done",
                    "usage": {
                        "prompt_tokens": response.prompt_tokens,
                        "completion_tokens": response.completion_tokens,
                        "total_tokens": response.total_tokens,
                    }
                }).to_string()));
            }
            Err(e) => {
                yield Ok(Event::default().data(json!({"type": "error", "message": e}).to_string()));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn get_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db_models = db::system::list_system_models(&state.pool).await?;
    let mut result = serde_json::Map::new();
    
    for m in db_models {
        result.entry(m.provider)
            .or_insert(json!(Vec::<String>::new()))
            .as_array_mut()
            .unwrap()
            .push(json!(m.model_name));
    }
    
    // If DB is empty, fallback to hardcoded
    if result.is_empty() {
        let providers = provider::supported_providers();
        for p in providers {
            let models = provider::available_models(p);
            result.insert(p.to_string(), json!(models));
        }
    }
    
    Ok(Json(json!(result)))
}
