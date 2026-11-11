use axum::http::StatusCode;
use axum::Router;
use serde_json::{json, Value};

use super::helpers::*;

async fn mcp_call(app: &Router, key: &str, id: i64, name: &str, arguments: Value) -> Value {
    let (status, body) = send(
        app,
        request(
            "POST",
            "/mcp",
            Some(key),
            None,
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    json(&body)
}

#[tokio::test]
async fn mcp_lists_tools_and_reads_context() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "mcp@example.com").await;
    let (_, key) = create_api_key(&env.pool, user_id, json!({"layer1": true})).await;
    let app = env.app();

    let (status, body) = send(
        &app,
        request(
            "POST",
            "/mcp",
            Some(&key),
            None,
            Some(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json(&body)["result"]["serverInfo"]["name"], "cerebrun-mcp");

    let (status, body) = send(
        &app,
        request(
            "POST",
            "/mcp",
            Some(&key),
            None,
            Some(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = json(&body)["result"]["tools"].as_array().unwrap().clone();
    assert_eq!(tools.len(), 22);
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"get_context"));
    assert!(
        names.contains(&"get_vault_data"),
        "consent flow tool must be discoverable"
    );

    let response = mcp_call(&app, &key, 3, "get_context", json!({"layer": 0})).await;
    assert!(!mcp_error(&response));
    let context = mcp_text(&response);
    assert_eq!(context["language"], "en");

    let response = mcp_call(&app, &key, 4, "get_context", json!({"layer": 1})).await;
    assert!(!mcp_error(&response));

    env.cleanup().await;
}

#[tokio::test]
async fn mcp_debates_are_scoped_to_their_owner() {
    let Some(env) = setup().await else {
        return;
    };
    let owner_id = create_user(&env.pool, "debate-owner@example.com").await;
    let other_id = create_user(&env.pool, "debate-other@example.com").await;
    let (_, owner_key) = create_api_key(&env.pool, owner_id, json!({"layer1": true})).await;
    let (_, other_key) = create_api_key(&env.pool, other_id, json!({"layer1": true})).await;
    let app = env.app();

    let response = mcp_call(
        &app,
        &owner_key,
        1,
        "create_debate",
        json!({
            "topic": "ownership test",
            "opening_message": "opening",
            "provider": "openai",
            "model": "test-model"
        }),
    )
    .await;
    assert!(!mcp_error(&response));
    let created = mcp_text(&response);
    let debate_id = created["debate_id"]
        .as_str()
        .expect("debate_id missing")
        .to_string();

    let response = mcp_call(
        &app,
        &other_key,
        2,
        "get_debate",
        json!({"debate_id": debate_id}),
    )
    .await;
    assert!(
        mcp_error(&response),
        "another user must not read the debate"
    );

    let response = mcp_call(
        &app,
        &owner_key,
        3,
        "get_debate",
        json!({"debate_id": debate_id}),
    )
    .await;
    assert!(!mcp_error(&response));
    assert_eq!(mcp_text(&response)["debate"]["status"], "open");

    env.cleanup().await;
}

#[tokio::test]
async fn vault_consent_flow_is_end_to_end_usable() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "vault@example.com").await;
    let session = create_session(&env.pool, user_id).await;
    let (_, key) = create_api_key(&env.pool, user_id, json!({"layer1": true})).await;
    let app = env.app();

    let (status, _) = send(
        &app,
        request(
            "PUT",
            "/api/v3/context",
            None,
            Some(&session),
            Some(json!({"OPENAI_API_KEY": "sk-test-secret"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let response = mcp_call(
        &app,
        &key,
        1,
        "request_vault_access",
        json!({"reason": "need the key", "requested_fields": ["OPENAI_API_KEY"]}),
    )
    .await;
    assert!(!mcp_error(&response));
    let consent = mcp_text(&response);
    let request_id = consent["request_id"]
        .as_str()
        .expect("request_id missing")
        .to_string();

    let (status, body) = send(
        &app,
        request(
            "POST",
            "/api/v3/approve",
            None,
            Some(&session),
            Some(json!({"request_id": request_id})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let vault_token = json(&body)["vault_token"]
        .as_str()
        .expect("vault_token missing")
        .to_string();

    let response = mcp_call(
        &app,
        &key,
        2,
        "get_vault_data",
        json!({"vault_token": vault_token}),
    )
    .await;
    assert!(!mcp_error(&response));
    assert_eq!(mcp_text(&response)["OPENAI_API_KEY"], "sk-test-secret");

    let response = mcp_call(
        &app,
        &key,
        3,
        "get_vault_data",
        json!({"vault_token": "vt_invalid"}),
    )
    .await;
    assert!(
        mcp_error(&response),
        "invalid consent tokens must be rejected"
    );

    let response = mcp_call(&app, &key, 4, "get_context", json!({"layer": 3})).await;
    assert!(
        mcp_error(&response),
        "direct vault access requires vault permission"
    );

    env.cleanup().await;
}
