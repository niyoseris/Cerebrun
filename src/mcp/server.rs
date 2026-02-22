use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedAgent;
use crate::crypto::vault as vault_crypto;
use crate::db;
use crate::error::AppError;
use crate::llm::provider;
use crate::models::{Layer0Update, Layer1Update, Layer2Update};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

pub async fn handle_mcp(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Json(req): Json<McpRequest>,
) -> Result<Json<McpResponse>, AppError> {
    let response = match req.method.as_str() {
        "initialize" => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "cerebrun-mcp",
                    "version": "0.3.0"
                }
            })),
            error: None,
        },
        "tools/list" => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(super::tools::list_tools()),
            error: None,
        },
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            let tool_result = execute_tool(&state, &agent, tool_name, arguments).await;

            match tool_result {
                Ok(result) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    })),
                    error: None,
                },
                Err(e) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Error: {}", e)
                        }],
                        "isError": true
                    })),
                    error: None,
                },
            }
        }
        _ => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
            }),
        },
    };

    Ok(Json(response))
}

fn build_system_prompt_layer0_only(
    layer0: Option<&crate::models::Layer0Public>,
) -> String {
    let mut parts = Vec::new();

    if let Some(l0) = layer0 {
        let mut prefs = Vec::new();
        if let Some(lang) = &l0.language { prefs.push(format!("Language: {}", lang)); }
        if let Some(tz) = &l0.timezone { prefs.push(format!("Timezone: {}", tz)); }
        if let Some(style) = &l0.communication_style { prefs.push(format!("Communication style: {}", style)); }
        if !prefs.is_empty() { parts.push(format!("[User Preferences]\n{}", prefs.join("\n"))); }
    }

    if parts.is_empty() { return String::new(); }
    format!("You are a helpful assistant. Here is context about the user:\n\n{}", parts.join("\n\n"))
}

async fn get_embedding_key(state: &AppState, user_id: Uuid) -> Option<(String, String)> {
    for prov in &["ollama", "openai"] {
        if let Ok(Some(key_record)) = db::llm_keys::get_provider_key(&state.pool, user_id, prov).await {
            let vault_key = vault_crypto::derive_vault_key(&state.config.session_secret);
            if let Ok(decrypted) = vault_crypto::decrypt_vault_data(&key_record.encrypted_key, &vault_key) {
                if let Ok(api_key) = String::from_utf8(decrypted) {
                    return Some((prov.to_string(), api_key));
                }
            }
        }
    }
    None
}

async fn execute_tool(
    state: &AppState,
    agent: &AuthenticatedAgent,
    tool_name: &str,
    arguments: Value,
) -> Result<Value, String> {
    let user_id = agent.api_key.user_id;

    match tool_name {
        "get_context" => {
            let layer = arguments
                .get("layer")
                .and_then(|v| v.as_i64())
                .ok_or("Missing 'layer' argument")?;

            match layer {
                0 => {
                    let data = db::layers::get_layer0(&state.pool, user_id)
                        .await.map_err(|e| e.to_string())?
                        .ok_or("Layer 0 data not found")?;
                    let _ = db::audit::log_access(
                        &state.pool, user_id, Some(agent.api_key.id),
                        "mcp_read_layer0", Some("0"), true, None, None, None,
                    ).await;
                    Ok(json!({
                        "language": data.language,
                        "timezone": data.timezone,
                        "output_format_preferences": data.output_format,
                        "blocked_topics": data.blocked_topics,
                        "communication_style": data.communication_style,
                    }))
                }
                1 => {
                    if !agent.has_permission("layer1") {
                        return Err("No permission for Layer 1".to_string());
                    }
                    let data = db::layers::get_layer1(&state.pool, user_id)
                        .await.map_err(|e| e.to_string())?
                        .ok_or("Layer 1 data not found")?;
                    let _ = db::audit::log_access(
                        &state.pool, user_id, Some(agent.api_key.id),
                        "mcp_read_layer1", Some("1"), true, None, None, None,
                    ).await;
                    Ok(json!({
                        "active_projects": data.active_projects,
                        "recent_conversations": data.recent_conversations,
                        "working_directories": data.working_directories,
                        "current_goals": data.current_goals,
                        "pinned_memories": data.pinned_memories,
                    }))
                }
                2 => {
                    if !agent.has_permission("layer2") {
                        return Err("No permission for Layer 2".to_string());
                    }
                    let data = db::layers::get_layer2(&state.pool, user_id)
                        .await.map_err(|e| e.to_string())?
                        .ok_or("Layer 2 data not found")?;
                    let _ = db::audit::log_access(
                        &state.pool, user_id, Some(agent.api_key.id),
                        "mcp_read_layer2", Some("2"), true, None, None, None,
                    ).await;
                    Ok(json!({
                        "display_name": data.display_name,
                        "location": data.location,
                        "interests": data.interests,
                        "contact_preferences": data.contact_preferences,
                        "relationship_notes": data.relationship_notes,
                    }))
                }
                3 => Err("Vault access requires explicit consent flow. Use request_vault_access tool first.".to_string()),
                _ => Err(format!("Invalid layer: {}", layer)),
            }
        }
        "update_context" => {
            let layer = arguments.get("layer").and_then(|v| v.as_i64()).ok_or("Missing 'layer' argument")?;
            let data = arguments.get("data").ok_or("Missing 'data' argument")?;

            match layer {
                0 => {
                    let update: Layer0Update = serde_json::from_value(data.clone()).map_err(|e| format!("Invalid data: {}", e))?;
                    let result = db::layers::update_layer0(&state.pool, user_id, &update).await.map_err(|e| e.to_string())?;
                    let _ = db::audit::log_access(&state.pool, user_id, Some(agent.api_key.id), "mcp_update_layer0", Some("0"), true, None, None, None).await;
                    Ok(json!({"status": "updated", "language": result.language, "timezone": result.timezone}))
                }
                1 => {
                    if !agent.has_permission("layer1") { return Err("No permission for Layer 1".to_string()); }
                    let update: Layer1Update = serde_json::from_value(data.clone()).map_err(|e| format!("Invalid data: {}", e))?;
                    let result = db::layers::update_layer1(&state.pool, user_id, &update).await.map_err(|e| e.to_string())?;

                    if let Some((emb_provider, emb_key)) = get_embedding_key(state, user_id).await {
                        let mut texts = Vec::new();
                        if let Some(goals) = &result.current_goals {
                            if let Some(arr) = goals.as_array() {
                                for (i, g) in arr.iter().enumerate() {
                                    if let Some(s) = g.as_str() {
                                        texts.push(("layer1".to_string(), format!("goal_{}", i), s.to_string()));
                                    }
                                }
                            }
                        }
                        if let Some(memories) = &result.pinned_memories {
                            if let Some(arr) = memories.as_array() {
                                for (i, m) in arr.iter().enumerate() {
                                    if let Some(s) = m.as_str() {
                                        texts.push(("layer1".to_string(), format!("memory_{}", i), s.to_string()));
                                    }
                                }
                            }
                        }
                        for (src_type, src_key, text) in texts {
                            if let Ok(emb) = provider::get_embedding(&emb_provider, &emb_key, &text).await {
                                let _ = db::embeddings::upsert_context_embedding(
                                    &state.pool, user_id, &src_type, &src_key, &text, &emb.embedding,
                                ).await;
                            }
                        }
                    }

                    let _ = db::audit::log_access(&state.pool, user_id, Some(agent.api_key.id), "mcp_update_layer1", Some("1"), true, None, None, None).await;
                    Ok(json!({"status": "updated"}))
                }
                2 => {
                    if !agent.has_permission("layer2") { return Err("No permission for Layer 2".to_string()); }
                    let update: Layer2Update = serde_json::from_value(data.clone()).map_err(|e| format!("Invalid data: {}", e))?;
                    let _ = db::layers::update_layer2(&state.pool, user_id, &update).await.map_err(|e| e.to_string())?;
                    let _ = db::audit::log_access(&state.pool, user_id, Some(agent.api_key.id), "mcp_update_layer2", Some("2"), true, None, None, None).await;
                    Ok(json!({"status": "updated"}))
                }
                _ => Err(format!("Cannot update layer {}", layer)),
            }
        }
        "request_vault_access" => {
            let reason = arguments.get("reason").and_then(|v| v.as_str()).ok_or("Missing 'reason' argument")?;
            let requested_fields = arguments.get("requested_fields").ok_or("Missing 'requested_fields' argument")?;

            let consent = db::vault::create_consent_request(
                &state.pool, user_id, agent.api_key.id, reason, requested_fields,
            ).await.map_err(|e| e.to_string())?;

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_vault_access_request", Some("3"), true, None, None, None,
            ).await;

            Ok(json!({
                "request_id": consent.id,
                "status": "pending",
                "message": "Vault access request sent. User must approve before data can be accessed."
            }))
        }

        "search_context" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for context search".to_string());
            }

            let query = arguments.get("query").and_then(|v| v.as_str())
                .ok_or("Missing 'query' argument")?;
            let limit = arguments.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
            let min_similarity = arguments.get("min_similarity").and_then(|v| v.as_f64()).unwrap_or(0.3);
            let include_knowledge = arguments.get("include_knowledge").and_then(|v| v.as_bool()).unwrap_or(true);

            let (emb_provider, emb_key) = get_embedding_key(state, user_id).await
                .ok_or("No embedding-capable provider key found. Add an OpenAI or Ollama key first.")?;

            let query_emb = provider::get_embedding(&emb_provider, &emb_key, query)
                .await
                .map_err(|e| format!("Embedding generation failed: {}", e))?;

            let mut results = Vec::new();

            let context_results = db::embeddings::search_similar_context(
                &state.pool, user_id, &query_emb.embedding, limit, min_similarity,
            ).await.map_err(|e| e.to_string())?;

            for r in &context_results {
                results.push(json!({
                    "type": "context",
                    "source_type": r.source_type,
                    "source_key": r.source_key,
                    "content": r.content_text,
                    "similarity": r.similarity,
                }));
            }

            if include_knowledge {
                let knowledge_results = db::embeddings::search_similar_knowledge(
                    &state.pool, user_id, &query_emb.embedding, limit, min_similarity,
                ).await.map_err(|e| e.to_string())?;

                for r in &knowledge_results {
                    results.push(json!({
                        "type": "knowledge",
                        "id": r.id,
                        "content": r.content,
                        "summary": r.summary,
                        "category": r.category,
                        "tags": r.tags,
                        "source_project": r.source_project,
                        "similarity": r.similarity,
                    }));
                }
            }

            results.sort_by(|a, b| {
                let sa = a["similarity"].as_f64().unwrap_or(0.0);
                let sb = b["similarity"].as_f64().unwrap_or(0.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_search_context", Some("search"), true, None, None,
                Some(&json!({"query": query, "results_count": results.len()})),
            ).await;

            Ok(json!({
                "results": results,
                "count": results.len(),
                "query": query,
                "embedding_provider": emb_provider,
            }))
        }

        "list_conversations" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for conversation history".to_string());
            }

            let limit = arguments.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            let provider_filter = arguments.get("provider").and_then(|v| v.as_str());

            let convs = db::conversations::get_recent_conversations(&state.pool, user_id, limit)
                .await.map_err(|e| e.to_string())?;

            let filtered: Vec<_> = if let Some(prov) = provider_filter {
                convs.into_iter().filter(|c| c.provider == prov).collect()
            } else {
                convs
            };

            let result: Vec<Value> = filtered.iter().map(|c| {
                json!({
                    "id": c.id,
                    "title": c.title,
                    "provider": c.provider,
                    "model": c.model,
                    "created_at": c.created_at,
                    "updated_at": c.updated_at,
                    "forked_from": c.forked_from,
                })
            }).collect();

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_list_conversations", Some("llm"), true, None, None, None,
            ).await;

            Ok(json!({ "conversations": result, "count": result.len() }))
        }

        "get_conversation" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for conversation history".to_string());
            }

            let conv_id_str = arguments.get("conversation_id").and_then(|v| v.as_str())
                .ok_or("Missing 'conversation_id' argument")?;
            let conv_id: Uuid = conv_id_str.parse().map_err(|_| "Invalid UUID format")?;

            let conv = db::conversations::get_conversation(&state.pool, conv_id, user_id)
                .await.map_err(|e| e.to_string())?
                .ok_or("Conversation not found")?;

            let messages = db::conversations::get_messages(&state.pool, conv_id)
                .await.map_err(|e| e.to_string())?;

            let msg_list: Vec<Value> = messages.iter().map(|m| {
                json!({
                    "id": m.id,
                    "role": m.role,
                    "content": m.content,
                    "provider": m.provider,
                    "model": m.model,
                    "tokens": m.total_tokens,
                    "created_at": m.created_at,
                })
            }).collect();

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_read_conversation", Some("llm"), true, None, None, None,
            ).await;

            Ok(json!({
                "conversation": {
                    "id": conv.id,
                    "title": conv.title,
                    "provider": conv.provider,
                    "model": conv.model,
                    "created_at": conv.created_at,
                    "updated_at": conv.updated_at,
                    "forked_from": conv.forked_from,
                },
                "messages": msg_list,
                "message_count": msg_list.len(),
            }))
        }

        "search_conversations" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for conversation search".to_string());
            }

            let query = arguments.get("query").and_then(|v| v.as_str())
                .ok_or("Missing 'query' argument")?;
            let provider_filter = arguments.get("provider").and_then(|v| v.as_str());
            let limit = arguments.get("limit").and_then(|v| v.as_i64()).unwrap_or(5);

            let results = db::conversations::search_conversations(
                &state.pool, user_id, query, provider_filter, limit,
            ).await.map_err(|e| e.to_string())?;

            let result_list: Vec<Value> = results.iter().map(|(conv, msgs)| {
                let matching_msgs: Vec<Value> = msgs.iter()
                    .filter(|m| m.content.to_lowercase().contains(&query.to_lowercase()))
                    .take(3)
                    .map(|m| json!({
                        "role": m.role,
                        "content_preview": if m.content.len() > 200 { format!("{}...", &m.content[..200]) } else { m.content.clone() },
                        "created_at": m.created_at,
                    }))
                    .collect();

                json!({
                    "conversation": {
                        "id": conv.id,
                        "title": conv.title,
                        "provider": conv.provider,
                        "model": conv.model,
                        "created_at": conv.created_at,
                        "updated_at": conv.updated_at,
                    },
                    "matching_messages": matching_msgs,
                    "total_messages": msgs.len(),
                })
            }).collect();

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_search_conversations", Some("llm"), true, None, None, None,
            ).await;

            Ok(json!({ "results": result_list, "count": result_list.len(), "query": query }))
        }

        "chat_with_llm" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for LLM Gateway".to_string());
            }

            let message = arguments.get("message").and_then(|v| v.as_str())
                .ok_or("Missing 'message' argument")?;
            let prov = arguments.get("provider").and_then(|v| v.as_str())
                .ok_or("Missing 'provider' argument")?;
            let mdl = arguments.get("model").and_then(|v| v.as_str())
                .ok_or("Missing 'model' argument")?;
            let conv_id_str = arguments.get("conversation_id").and_then(|v| v.as_str());
            let title = arguments.get("title").and_then(|v| v.as_str());

            let provider_key = db::llm_keys::get_provider_key(&state.pool, user_id, prov)
                .await.map_err(|e| e.to_string())?
                .ok_or_else(|| format!("No API key configured for provider: {}. Add one in the dashboard.", prov))?;

            let vault_key = vault_crypto::derive_vault_key(&state.config.session_secret);
            let api_key = vault_crypto::decrypt_vault_data(&provider_key.encrypted_key, &vault_key)
                .map_err(|e| format!("Key decryption failed: {}", e))?;
            let api_key_str = String::from_utf8(api_key).map_err(|e| e.to_string())?;

            let conv_id = if let Some(id_str) = conv_id_str {
                let id: Uuid = id_str.parse().map_err(|_| "Invalid conversation_id UUID")?;
                let _ = db::conversations::get_conversation(&state.pool, id, user_id)
                    .await.map_err(|e| e.to_string())?
                    .ok_or("Conversation not found")?;
                id
            } else {
                let layer0 = db::layers::get_layer0(&state.pool, user_id).await.map_err(|e| e.to_string())?;
                let sys_prompt = build_system_prompt_layer0_only(layer0.as_ref());

                let conv = db::conversations::create_conversation(
                    &state.pool, user_id, prov, mdl,
                    title.or(Some(&format!("MCP: {}", &message[..message.len().min(50)]))),
                    if sys_prompt.is_empty() { None } else { Some(&sys_prompt) },
                    None, None,
                ).await.map_err(|e| e.to_string())?;
                conv.id
            };

            let _ = db::conversations::add_message(
                &state.pool, conv_id, "user", message, None, None, 0, 0, 0,
            ).await.map_err(|e| e.to_string())?;

            let history = db::conversations::get_messages(&state.pool, conv_id)
                .await.map_err(|e| e.to_string())?;

            let conv = db::conversations::get_conversation(&state.pool, conv_id, user_id)
                .await.map_err(|e| e.to_string())?
                .ok_or("Conversation disappeared")?;

            let mut llm_messages: Vec<provider::LlmMessage> = Vec::new();
            if let Some(sys) = &conv.system_prompt {
                if !sys.is_empty() {
                    llm_messages.push(provider::LlmMessage { role: "system".to_string(), content: sys.clone() });
                }
            }
            for msg in &history {
                llm_messages.push(provider::LlmMessage { role: msg.role.clone(), content: msg.content.clone() });
            }

            let response = provider::call_llm(prov, mdl, &api_key_str, &llm_messages)
                .await.map_err(|e| format!("LLM call failed: {}", e))?;

            let assistant_msg = db::conversations::add_message(
                &state.pool, conv_id, "assistant", &response.content,
                Some(prov), Some(mdl),
                response.prompt_tokens, response.completion_tokens, response.total_tokens,
            ).await.map_err(|e| e.to_string())?;

            let _ = db::llm_usage::record_usage(
                &state.pool, user_id, Some(conv_id), Some(assistant_msg.id),
                prov, mdl, response.prompt_tokens, response.completion_tokens,
                response.total_tokens,
            ).await;

            let meta = json!({ "provider": prov, "model": mdl, "tokens": response.total_tokens });
            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_chat_with_llm", Some("llm"), true, None, None,
                Some(&meta),
            ).await;

            Ok(json!({
                "conversation_id": conv_id,
                "response": response.content,
                "provider": prov,
                "model": mdl,
                "usage": {
                    "prompt_tokens": response.prompt_tokens,
                    "completion_tokens": response.completion_tokens,
                    "total_tokens": response.total_tokens,
                }
            }))
        }

        "fork_conversation" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for conversation forking".to_string());
            }

            let conv_id_str = arguments.get("conversation_id").and_then(|v| v.as_str())
                .ok_or("Missing 'conversation_id'")?;
            let conv_id: Uuid = conv_id_str.parse().map_err(|_| "Invalid conversation_id UUID")?;
            let msg_id_str = arguments.get("message_id").and_then(|v| v.as_str())
                .ok_or("Missing 'message_id'")?;
            let msg_id: Uuid = msg_id_str.parse().map_err(|_| "Invalid message_id UUID")?;
            let new_provider = arguments.get("new_provider").and_then(|v| v.as_str())
                .ok_or("Missing 'new_provider'")?;
            let new_model = arguments.get("new_model").and_then(|v| v.as_str())
                .ok_or("Missing 'new_model'")?;

            let original = db::conversations::get_conversation(&state.pool, conv_id, user_id)
                .await.map_err(|e| e.to_string())?
                .ok_or("Conversation not found")?;

            let messages_to_copy = db::conversations::get_messages_up_to(&state.pool, conv_id, msg_id)
                .await.map_err(|e| e.to_string())?;

            let layer0 = db::layers::get_layer0(&state.pool, user_id).await.map_err(|e| e.to_string())?;
            let sys_prompt = build_system_prompt_layer0_only(layer0.as_ref());

            let new_conv = db::conversations::create_conversation(
                &state.pool, user_id, new_provider, new_model,
                Some(&format!("Fork: {} -> {}", original.model, new_model)),
                if sys_prompt.is_empty() { None } else { Some(&sys_prompt) },
                Some(conv_id), Some(msg_id),
            ).await.map_err(|e| e.to_string())?;

            for msg in &messages_to_copy {
                let _ = db::conversations::add_message(
                    &state.pool, new_conv.id, &msg.role, &msg.content,
                    msg.provider.as_deref(), msg.model.as_deref(),
                    msg.prompt_tokens.unwrap_or(0), msg.completion_tokens.unwrap_or(0),
                    msg.total_tokens.unwrap_or(0),
                ).await.map_err(|e| e.to_string())?;
            }

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_fork_conversation", Some("llm"), true, None, None, None,
            ).await;

            Ok(json!({
                "new_conversation_id": new_conv.id,
                "forked_from": conv_id,
                "fork_point_message_id": msg_id,
                "provider": new_provider,
                "model": new_model,
                "messages_copied": messages_to_copy.len(),
            }))
        }

        "get_llm_usage" => {
            let summary = db::llm_usage::get_usage_summary(&state.pool, user_id)
                .await.map_err(|e| e.to_string())?;

            let total_tokens: i64 = summary.iter().map(|s| s.total_tokens.unwrap_or(0)).sum();

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_get_llm_usage", Some("llm"), true, None, None, None,
            ).await;

            Ok(json!({
                "total_tokens": total_tokens,
                "by_provider": summary,
            }))
        }

        "push_knowledge" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for Knowledge Base".to_string());
            }

            let content = arguments.get("content").and_then(|v| v.as_str())
                .ok_or("Missing 'content' argument")?;
            let summary = arguments.get("summary").and_then(|v| v.as_str());
            let category = arguments.get("category").and_then(|v| v.as_str()).unwrap_or("uncategorized");
            let subcategory = arguments.get("subcategory").and_then(|v| v.as_str());
            let tags: Vec<String> = arguments.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let source_project = arguments.get("source_project").and_then(|v| v.as_str());

            let source_agent = Some(agent.api_key.name.as_str());

            let entry = db::knowledge::insert_knowledge(
                &state.pool, user_id, content, summary, category, subcategory,
                &tags, source_agent, source_project, None,
            ).await.map_err(|e| e.to_string())?;

            if let Some((emb_provider, emb_key)) = get_embedding_key(state, user_id).await {
                let embed_text = if let Some(s) = summary {
                    format!("{}: {}", s, content)
                } else {
                    content.to_string()
                };
                if let Ok(emb) = provider::get_embedding(&emb_provider, &emb_key, &embed_text).await {
                    let _ = db::embeddings::update_knowledge_embedding(
                        &state.pool, entry.id, &emb.embedding,
                    ).await;
                }
            }

            let meta = json!({ "category": category, "tags": tags, "source_project": source_project });
            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_push_knowledge", Some("knowledge"), true, None, None, Some(&meta),
            ).await;

            Ok(json!({
                "id": entry.id,
                "status": "stored",
                "category": entry.category,
                "summary": entry.summary,
                "tags": entry.tags,
                "source_agent": entry.source_agent,
                "created_at": entry.created_at,
            }))
        }

        "query_knowledge" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for Knowledge Base".to_string());
            }

            let keyword = arguments.get("keyword").and_then(|v| v.as_str());
            let category = arguments.get("category").and_then(|v| v.as_str());
            let tag = arguments.get("tag").and_then(|v| v.as_str());
            let source_project = arguments.get("source_project").and_then(|v| v.as_str());
            let limit = arguments.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            let offset = arguments.get("offset").and_then(|v| v.as_i64()).unwrap_or(0);

            let entries = db::knowledge::query_knowledge(
                &state.pool, user_id, keyword, category, tag, source_project, limit, offset,
            ).await.map_err(|e| e.to_string())?;

            let results: Vec<Value> = entries.iter().map(|e| {
                json!({
                    "id": e.id,
                    "content": e.content,
                    "summary": e.summary,
                    "category": e.category,
                    "subcategory": e.subcategory,
                    "tags": e.tags,
                    "source_agent": e.source_agent,
                    "source_project": e.source_project,
                    "created_at": e.created_at,
                })
            }).collect();

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_query_knowledge", Some("knowledge"), true, None, None, None,
            ).await;

            Ok(json!({ "entries": results, "count": results.len() }))
        }

        "list_knowledge_categories" => {
            if !agent.has_permission("layer1") {
                return Err("No permission: layer1 access required for Knowledge Base".to_string());
            }

            let categories = db::knowledge::list_categories(&state.pool, user_id)
                .await.map_err(|e| e.to_string())?;

            let total = db::knowledge::count_knowledge(&state.pool, user_id, None, None, None, None)
                .await.map_err(|e| e.to_string())?;

            let _ = db::audit::log_access(
                &state.pool, user_id, Some(agent.api_key.id),
                "mcp_list_knowledge_categories", Some("knowledge"), true, None, None, None,
            ).await;

            Ok(json!({
                "categories": categories,
                "total_entries": total,
            }))
        }

        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}
