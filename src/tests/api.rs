use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use super::helpers::*;

#[tokio::test]
async fn health_reports_healthy() {
    let Some(env) = setup().await else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let app = env.app();

    let (status, body) = send(&app, request("GET", "/health", None, None, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json(&body)["status"], "healthy");

    env.cleanup().await;
}

#[tokio::test]
async fn session_can_read_and_write_context_layers() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "layers@example.com").await;
    let session = create_session(&env.pool, user_id).await;
    let app = env.app();

    for uri in ["/api/v0/context", "/api/v1/context", "/api/v2/context"] {
        let (status, _) = send(&app, request("GET", uri, None, Some(&session), None)).await;
        assert_eq!(status, StatusCode::OK, "{uri} should accept a session");
    }

    let (status, _) = send(
        &app,
        request(
            "PUT",
            "/api/v1/context",
            None,
            Some(&session),
            Some(json!({"current_goals": ["ship the product"], "pinned_memories": ["rule 1"]})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = send(
        &app,
        request("GET", "/api/v1/context", None, Some(&session), None),
    )
    .await;
    let value = json(&body);
    assert_eq!(value["current_goals"][0], "ship the product");
    assert_eq!(value["pinned_memories"][0], "rule 1");

    env.cleanup().await;
}

#[tokio::test]
async fn api_key_permissions_are_enforced() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "permissions@example.com").await;
    let (_, basic_key) = create_api_key(&env.pool, user_id, json!({"layer1": false})).await;
    let (_, layer1_key) = create_api_key(&env.pool, user_id, json!({"layer1": true})).await;
    let app = env.app();

    let (status, _) = send(
        &app,
        request("GET", "/api/v0/context", Some(&basic_key), None, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        &app,
        request("GET", "/api/v1/context", Some(&basic_key), None, None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = send(
        &app,
        request("GET", "/api/v1/context", Some(&layer1_key), None, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    env.cleanup().await;
}

#[tokio::test]
async fn api_key_can_be_created_and_revoked_from_session() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "keys@example.com").await;
    let session = create_session(&env.pool, user_id).await;
    let app = env.app();

    let (status, body) = send(
        &app,
        request(
            "POST",
            "/api/keys",
            None,
            Some(&session),
            Some(json!({"name": "dashboard-key", "permissions": {"layer1": true}})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let created = json(&body);
    let raw_key = created["key"].as_str().expect("key missing").to_string();
    let key_id = created["id"].as_str().expect("id missing").to_string();

    let (status, _) = send(
        &app,
        request("GET", "/api/v0/context", Some(&raw_key), None, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        &app,
        request(
            "POST",
            &format!("/api/keys/{}/revoke", key_id),
            None,
            Some(&session),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        &app,
        request("GET", "/api/v0/context", Some(&raw_key), None, None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    env.cleanup().await;
}

#[tokio::test]
async fn knowledge_categories_and_soft_delete_are_consistent() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "knowledge@example.com").await;
    let session = create_session(&env.pool, user_id).await;
    let app = env.app();

    let (status, body) = send(
        &app,
        request(
            "POST",
            "/api/knowledge",
            None,
            Some(&session),
            Some(json!({"content": "__test_knowledge__", "category": "note", "tags": ["test"]})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = json(&body)["id"]
        .as_str()
        .expect("knowledge id missing")
        .to_string();

    let (status, body) = send(
        &app,
        request(
            "GET",
            "/api/knowledge/categories",
            None,
            Some(&session),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let categories = json(&body);
    assert!(categories
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["category"] == "note"));

    let (status, _) = send(
        &app,
        request(
            "DELETE",
            &format!("/api/knowledge/{}", id),
            None,
            Some(&session),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        &app,
        request(
            "GET",
            "/api/knowledge?keyword=__test_knowledge__",
            None,
            Some(&session),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json(&body)["total"],
        0,
        "soft-deleted entry must not be listed"
    );

    let (_, body) = send(
        &app,
        request(
            "GET",
            "/api/knowledge/categories",
            None,
            Some(&session),
            None,
        ),
    )
    .await;
    let categories = json(&body);
    assert!(
        categories
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["category"] != "note"),
        "soft-deleted entry must not be counted in categories"
    );

    env.cleanup().await;
}

#[tokio::test]
async fn export_includes_knowledge_and_conversations() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "export@example.com").await;
    let session = create_session(&env.pool, user_id).await;
    let app = env.app();

    sqlx::query(
        "INSERT INTO knowledge_entries (user_id, content, category) VALUES ($1, $2, 'note')",
    )
    .bind(user_id)
    .bind("exported knowledge")
    .execute(&env.pool)
    .await
    .unwrap();

    let (conv_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO conversations (user_id, title, provider, model) VALUES ($1, 'exported conversation', 'openai', 'test') RETURNING id",
    )
    .bind(user_id)
    .fetch_one(&env.pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO conversation_messages (conversation_id, role, content) VALUES ($1, 'user', 'hello')",
    )
    .bind(conv_id)
    .execute(&env.pool)
    .await
    .unwrap();

    let (status, body) = send(
        &app,
        request("POST", "/api/export", None, Some(&session), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let value = json(&body);
    assert_eq!(value["knowledge"].as_array().unwrap().len(), 1);
    assert_eq!(value["conversations"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["conversations"][0]["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(value.get("vault").is_some());

    env.cleanup().await;
}

#[tokio::test]
async fn public_debate_page_renders_data_and_escapes_html() {
    let Some(env) = setup().await else {
        return;
    };
    let user_id = create_user(&env.pool, "debate@example.com").await;
    let public_id = Uuid::new_v4();

    let (debate_id,): (Uuid,) = sqlx::query_as(
        r#"INSERT INTO debates (user_id, public_id, topic, description, completion_criteria)
           VALUES ($1, $2, $3, $4, '["criterion one"]'::jsonb) RETURNING id"#,
    )
    .bind(user_id)
    .bind(public_id)
    .bind("Test <script>alert(1)</script> topic")
    .bind("<b>description</b>")
    .fetch_one(&env.pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO debate_messages (debate_id, provider, model, round, role, content) VALUES ($1, 'openai', 'test-model', 1, 'opening', $2)",
    )
    .bind(debate_id)
    .bind("Message <img src=x onerror=alert(2)> body")
    .execute(&env.pool)
    .await
    .unwrap();

    let app = env.app();
    let (status, body) = send(
        &app,
        request("GET", &format!("/debate/{}", public_id), None, None, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("render(DEBATE_DATA)"));
    assert!(
        html.contains("\\u003cscript\\u003e"),
        "JSON must be script-safe"
    );
    assert!(
        !html.contains("<script>alert(1)</script>"),
        "topic must not break out of the script tag"
    );
    assert!(
        html.contains("&lt;script&gt;"),
        "title must be HTML-escaped"
    );

    env.cleanup().await;
}

#[tokio::test]
async fn setup_page_is_public_and_contains_mcp_configs() {
    let Some(env) = setup().await else {
        return;
    };
    let app = env.app();

    let (status, body) = send(&app, request("GET", "/setup", None, None, None)).await;
    assert_eq!(status, StatusCode::OK);

    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Connect an AI agent"));
    assert!(html.contains("mcpServers"));
    assert!(html.contains("/mcp"));
    assert!(html.contains("Authorization"));

    env.cleanup().await;
}

#[tokio::test]
async fn fresh_schema_has_required_tables_and_columns() {
    let Some(env) = setup().await else {
        return;
    };

    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT tablename FROM pg_tables WHERE schemaname = current_schema() ORDER BY tablename",
    )
    .fetch_all(&env.pool)
    .await
    .unwrap();
    let names: Vec<String> = tables.into_iter().map(|t| t.0).collect();

    for required in [
        "users",
        "sessions",
        "api_keys",
        "layer0_public",
        "layer1_context",
        "layer2_personal",
        "layer3_vault",
        "knowledge_entries",
        "context_embeddings",
        "conversations",
        "conversation_messages",
        "debates",
        "debate_messages",
        "debate_participants",
        "content_versions",
    ] {
        assert!(
            names.contains(&required.to_string()),
            "missing table {required}"
        );
    }

    for (table, column) in [
        ("users", "is_admin"),
        ("conversations", "inject_context"),
        ("conversations", "context_token_budget"),
        ("conversations", "use_tools"),
        ("knowledge_entries", "deleted_at"),
        ("debate_messages", "deleted_at"),
    ] {
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT FROM information_schema.columns \
             WHERE table_schema = current_schema() AND table_name = $1 AND column_name = $2)",
        )
        .bind(table)
        .bind(column)
        .fetch_one(&env.pool)
        .await
        .unwrap();
        assert!(exists.0, "missing column {table}.{column}");
    }

    env.cleanup().await;
}
