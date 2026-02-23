mod api;
mod auth;
mod config;
mod crypto;
mod db;
mod error;
mod llm;
mod mcp;
mod models;

use axum::http::header;
use axum::routing::{delete, get, post, put};
use axum::Router;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: config::AppConfig,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = config::AppConfig::from_env();

    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    db::pool::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let state = AppState {
        pool,
        config: config.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::COOKIE,
            "X-Vault-Token".parse().unwrap(),
        ]);

    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/chat", get(serve_chat))
        .route("/auth/google", get(auth::oauth::google_auth))
        .route("/auth/google/callback", get(auth::oauth::google_callback))
        .route("/auth/logout", post(auth::oauth::logout))
        .route("/auth/me", get(api::layers::get_me))
        .route("/api/v0/context", get(api::layers::get_layer0))
        .route("/api/v0/context", put(api::layers::put_layer0))
        .route("/api/v1/context", get(api::layers::get_layer1))
        .route("/api/v1/context", put(api::layers::put_layer1))
        .route("/api/v2/context", get(api::layers::get_layer2))
        .route("/api/v2/context", put(api::layers::put_layer2))
        .route("/api/v3/request", post(api::vault::request_vault_access))
        .route("/api/v3/approve", post(api::vault::approve_vault_request))
        .route("/api/v3/deny", post(api::vault::deny_vault_request))
        .route("/api/v3/context", get(api::vault::get_vault_context))
        .route("/api/v3/context", put(api::vault::put_vault_context))
        .route("/api/keys", get(api::keys::list_keys))
        .route("/api/keys", post(api::keys::create_key))
        .route("/api/keys/:id", delete(api::keys::delete_key))
        .route("/api/keys/:id/revoke", post(api::keys::revoke_key))
        .route("/api/audit", get(api::audit::get_audit_log))
        .route("/api/consent/pending", get(api::vault::get_pending_consents))
        .route("/api/export", post(api::audit::export_data))
        .route("/api/account", delete(api::audit::delete_account))
        .route("/mcp", post(mcp::server::handle_mcp))
        .route("/api/llm/keys", get(api::llm::list_provider_keys))
        .route("/api/llm/keys", post(api::llm::add_provider_key))
        .route("/api/llm/keys/:id", delete(api::llm::delete_provider_key))
        .route("/api/llm/conversations", get(api::llm::list_conversations))
        .route("/api/llm/conversations", post(api::llm::create_conversation))
        .route("/api/llm/conversations/:id", get(api::llm::get_conversation_messages))
        .route("/api/llm/conversations/:id", delete(api::llm::delete_conversation))
        .route("/api/llm/conversations/:id/chat", post(api::llm::chat))
        .route("/api/llm/conversations/:id/stream", post(api::llm::stream_chat))
        .route("/api/llm/conversations/:id/fork", post(api::llm::fork_conversation))
        .route("/api/llm/compare", post(api::llm::compare))
        .route("/api/llm/metrics", get(api::llm::get_usage_metrics))
        .route("/api/llm/models", get(api::llm::get_models))
        .route("/api/admin/models", get(api::admin::list_system_models))
        .route("/api/admin/models", post(api::admin::add_system_model))
        .route("/api/admin/models/:id", delete(api::admin::delete_system_model))
        .route("/api/admin/settings", get(api::admin::get_settings))
        .route("/api/admin/settings", post(api::admin::update_setting))
        .route("/api/knowledge", get(api::knowledge::list_knowledge))
        .route("/api/knowledge", post(api::knowledge::create_knowledge))
        .route("/api/knowledge/:id", get(api::knowledge::get_knowledge))
        .route("/api/knowledge/:id", delete(api::knowledge::delete_knowledge))
        .route("/health", get(health_check))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.server_host, config.server_port)
        .parse()
        .expect("Invalid address");

    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "user-context-mcp",
        "version": "0.2.0"
    }))
}

async fn serve_dashboard() -> axum::response::Html<String> {
    let html = include_str!("../static/dashboard/index.html");
    axum::response::Html(html.to_string())
}

async fn serve_chat() -> axum::response::Html<String> {
    let html = include_str!("../static/dashboard/chat.html");
    axum::response::Html(html.to_string())
}
