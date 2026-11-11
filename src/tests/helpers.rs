use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::crypto::hash::sha256_hash;
use crate::AppState;

/// `CREATE EXTENSION` is database-wide and not concurrency-safe, so make sure
/// both extensions exist once before any test schema runs migrations.
static EXTENSIONS_READY: OnceCell<()> = OnceCell::const_new();

/// A disposable PostgreSQL schema plus the application state that points at it.
pub struct TestEnv {
    pub pool: PgPool,
    pub admin: PgPool,
    pub state: AppState,
    pub schema: String,
}

/// Build an isolated test environment.
///
/// Returns `None` when `TEST_DATABASE_URL` is not set, so `cargo test` still
/// works for the pure unit tests on a machine without PostgreSQL. CI always
/// sets the variable.
pub async fn setup() -> Option<TestEnv> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;

    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("TEST_DATABASE_URL must point at a reachable PostgreSQL database");

    EXTENSIONS_READY
        .get_or_init(|| async {
            for extension in ["pgcrypto", "vector"] {
                let _ = sqlx::query(&format!("CREATE EXTENSION IF NOT EXISTS {}", extension))
                    .execute(&admin)
                    .await;
            }
        })
        .await;

    let schema = format!("test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA \"{}\"", schema))
        .execute(&admin)
        .await
        .expect("failed to create test schema");

    let schema_for_connections = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |conn, _meta| {
            let schema = schema_for_connections.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO \"{}\", public", schema))
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("failed to connect to test database");

    crate::db::pool::run_migrations(&pool)
        .await
        .expect("migrations failed on test schema");

    let config = AppConfig {
        database_url: url.clone(),
        google_client_id: "test-client".to_string(),
        google_client_secret: "test-secret".to_string(),
        google_redirect_uri: "http://localhost:5050/auth/google/callback".to_string(),
        session_secret: "test-session-secret".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
    };

    let state = AppState {
        pool: pool.clone(),
        config,
    };

    Some(TestEnv {
        pool,
        admin,
        state,
        schema,
    })
}

impl TestEnv {
    pub fn app(&self) -> Router {
        crate::build_app(self.state.clone())
    }

    pub async fn cleanup(self) {
        let _ = sqlx::query(&format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
            self.schema
        ))
        .execute(&self.admin)
        .await;
        self.pool.close().await;
        self.admin.close().await;
    }
}

pub async fn create_user(pool: &PgPool, email: &str) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO users (google_id, email, display_name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(format!("google-{}", email))
    .bind(email)
    .bind("Test User")
    .fetch_one(pool)
    .await
    .expect("failed to create test user");

    for table in ["layer0_public", "layer1_context", "layer2_personal"] {
        sqlx::query(&format!(
            "INSERT INTO {} (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
            table
        ))
        .bind(id)
        .execute(pool)
        .await
        .expect("failed to create layer row");
    }

    id
}

pub async fn create_api_key(
    pool: &PgPool,
    user_id: Uuid,
    permissions: serde_json::Value,
) -> (Uuid, String) {
    let raw = format!("sk_live_{}", Uuid::new_v4().simple());
    let hash = sha256_hash(&raw);

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO api_keys (user_id, name, key_hash, permissions) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(user_id)
    .bind("test-key")
    .bind(&hash)
    .bind(&permissions)
    .fetch_one(pool)
    .await
    .expect("failed to create test api key");

    (id, raw)
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> String {
    let raw = format!("sess_{}", Uuid::new_v4().simple());
    let hash = sha256_hash(&raw);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 day')",
    )
    .bind(user_id)
    .bind(&hash)
    .execute(pool)
    .await
    .expect("failed to create test session");

    raw
}

pub async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router did not return a response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    (status, bytes.to_vec())
}

pub fn request(
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);

    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", token));
    }
    if let Some(token) = session {
        builder = builder.header(header::COOKIE, format!("session={}", token));
    }

    let body = match body {
        Some(value) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&value).expect("failed to serialize request body"))
        }
        None => Body::empty(),
    };

    builder.body(body).expect("failed to build request")
}

pub fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or(serde_json::Value::Null)
}

pub fn mcp_text(response: &serde_json::Value) -> serde_json::Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("null");
    serde_json::from_str(text).unwrap_or(serde_json::Value::Null)
}

pub fn mcp_error(response: &serde_json::Value) -> bool {
    response["result"]["isError"].as_bool().unwrap_or(false)
}
