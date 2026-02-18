use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::AuthenticatedAgent;
use crate::db;
use crate::error::AppError;
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
                    "name": "user-context-mcp",
                    "version": "0.1.0"
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
                        .await
                        .map_err(|e| e.to_string())?
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
                        .await
                        .map_err(|e| e.to_string())?
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
                        .await
                        .map_err(|e| e.to_string())?
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
            let layer = arguments
                .get("layer")
                .and_then(|v| v.as_i64())
                .ok_or("Missing 'layer' argument")?;
            let data = arguments.get("data").ok_or("Missing 'data' argument")?;

            match layer {
                0 => {
                    let update: Layer0Update = serde_json::from_value(data.clone())
                        .map_err(|e| format!("Invalid data: {}", e))?;
                    let result = db::layers::update_layer0(&state.pool, user_id, &update)
                        .await
                        .map_err(|e| e.to_string())?;
                    let _ = db::audit::log_access(
                        &state.pool, user_id, Some(agent.api_key.id),
                        "mcp_update_layer0", Some("0"), true, None, None, None,
                    ).await;
                    Ok(json!({"status": "updated", "language": result.language, "timezone": result.timezone}))
                }
                1 => {
                    if !agent.has_permission("layer1") {
                        return Err("No permission for Layer 1".to_string());
                    }
                    let update: Layer1Update = serde_json::from_value(data.clone())
                        .map_err(|e| format!("Invalid data: {}", e))?;
                    let _ = db::layers::update_layer1(&state.pool, user_id, &update)
                        .await
                        .map_err(|e| e.to_string())?;
                    let _ = db::audit::log_access(
                        &state.pool, user_id, Some(agent.api_key.id),
                        "mcp_update_layer1", Some("1"), true, None, None, None,
                    ).await;
                    Ok(json!({"status": "updated"}))
                }
                2 => {
                    if !agent.has_permission("layer2") {
                        return Err("No permission for Layer 2".to_string());
                    }
                    let update: Layer2Update = serde_json::from_value(data.clone())
                        .map_err(|e| format!("Invalid data: {}", e))?;
                    let _ = db::layers::update_layer2(&state.pool, user_id, &update)
                        .await
                        .map_err(|e| e.to_string())?;
                    let _ = db::audit::log_access(
                        &state.pool, user_id, Some(agent.api_key.id),
                        "mcp_update_layer2", Some("2"), true, None, None, None,
                    ).await;
                    Ok(json!({"status": "updated"}))
                }
                _ => Err(format!("Cannot update layer {}", layer)),
            }
        }
        "request_vault_access" => {
            let reason = arguments
                .get("reason")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'reason' argument")?;
            let requested_fields = arguments
                .get("requested_fields")
                .ok_or("Missing 'requested_fields' argument")?;

            let consent = db::vault::create_consent_request(
                &state.pool,
                user_id,
                agent.api_key.id,
                reason,
                requested_fields,
            )
            .await
            .map_err(|e| e.to_string())?;

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
        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}
