use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::Redirect;
use serde::Deserialize;

use crate::auth::session;
use crate::crypto::hash::{generate_random_key, sha256_hash};
use crate::db;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct OAuthCallback {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

use crate::AppState;

fn get_redirect_uri(headers: &HeaderMap, config: &crate::config::AppConfig) -> String {
    if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
        let final_scheme = if host.contains("localhost") || host.contains("127.0.0.1") { "http" } else { "https" };
        format!("{}://{}/auth/google/callback", final_scheme, host)
    } else {
        config.google_redirect_uri.clone()
    }
}

pub async fn google_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Redirect), AppError> {
    let csrf_state = generate_random_key("state");
    let state_hash = sha256_hash(&csrf_state);

    let redirect_uri = get_redirect_uri(&headers, &state.config);
    tracing::info!("Google Auth Start - Redirect URI: {}", redirect_uri);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ((SELECT id FROM users LIMIT 0), $1, NOW() + INTERVAL '10 minutes') ON CONFLICT DO NOTHING"
    ).bind(&format!("oauth_state:{}", state_hash))
    .execute(&state.pool)
    .await
    .ok();

    let mut resp_headers = HeaderMap::new();
    let cookie_value = format!(
        "oauth_state={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax",
        csrf_state
    );
    resp_headers.insert(
        http::header::SET_COOKIE,
        cookie_value.parse().unwrap(),
    );

    let redirect_cookie = format!(
        "oauth_redirect_uri={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax",
        redirect_uri
    );
    resp_headers.append(
        http::header::SET_COOKIE,
        redirect_cookie.parse().unwrap(),
    );

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&access_type=offline&prompt=consent",
        state.config.google_client_id,
        urlencoding(&redirect_uri),
        csrf_state
    );
    Ok((resp_headers, Redirect::temporary(&auth_url)))
}

pub async fn google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<OAuthCallback>,
) -> Result<(HeaderMap, Redirect), AppError> {
    let callback_state = params.state.as_deref()
        .ok_or_else(|| AppError::OAuth("Missing OAuth state parameter".to_string()))?;

    let cookie_header = headers
        .get(http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    tracing::info!("Google Callback - Cookies: {}", cookie_header);

    let stored_state = cookie_header
        .split(';')
        .filter_map(|s| s.trim().strip_prefix("oauth_state="))
        .next()
        .ok_or_else(|| AppError::OAuth("Missing OAuth state cookie".to_string()))?;

    if callback_state != stored_state {
        return Err(AppError::OAuth("OAuth state mismatch - possible CSRF attack".to_string()));
    }

    let redirect_uri = cookie_header
        .split(';')
        .filter_map(|s| s.trim().strip_prefix("oauth_redirect_uri="))
        .next()
        .map(|s| s.to_string())
        .unwrap_or_else(|| get_redirect_uri(&headers, &state.config));
    
    tracing::info!("Google Callback - Redirect URI: {}", redirect_uri);
    tracing::info!("Google Callback - State: {:?}", params.state);

    let token_response: GoogleTokenResponse = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", params.code.as_str()),
            ("client_id", state.config.google_client_id.as_str()),
            ("client_secret", state.config.google_client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("OAuth - Token request failed: {}", e);
            AppError::OAuth(e.to_string())
        })?
        .json()
        .await
        .map_err(|e| {
            tracing::error!("OAuth - Token JSON parsing failed: {}", e);
            AppError::OAuth(e.to_string())
        })?;

    tracing::info!("OAuth - Token Received Successfully");

    let user_info: GoogleUserInfo = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token_response.access_token)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("OAuth - Failed to fetch userinfo: {}", e);
            AppError::OAuth(e.to_string())
        })?
        .json()
        .await
        .map_err(|e| {
            tracing::error!("OAuth - Failed to parse userinfo JSON: {}", e);
            AppError::OAuth(e.to_string())
        })?;

    tracing::info!("OAuth - User Info Fetched: {}", user_info.email);

    let user = db::users::upsert_user(
        &state.pool,
        &user_info.sub,
        &user_info.email,
        user_info.name.as_deref(),
        user_info.picture.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("OAuth - Failed to upsert user: {}", e);
        e
    })?;

    let session_token = session::create_session(&state.pool, user.id).await?;

    let mut resp_headers = HeaderMap::new();
    let session_cookie = format!(
        "session={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax",
        session_token
    );
    resp_headers.insert(
        http::header::SET_COOKIE,
        session_cookie.parse().unwrap(),
    );
    resp_headers.append(
        http::header::SET_COOKIE,
        "oauth_state=; HttpOnly; Path=/; Max-Age=0".parse().unwrap(),
    );
    resp_headers.append(
        http::header::SET_COOKIE,
        "oauth_redirect_uri=; HttpOnly; Path=/; Max-Age=0".parse().unwrap(),
    );

    Ok((resp_headers, Redirect::temporary("/")))
}

fn urlencoding(s: &str) -> String {
    s.replace(":", "%3A")
        .replace("/", "%2F")
        .replace("?", "%3F")
        .replace("=", "%3D")
        .replace("&", "%26")
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(HeaderMap, axum::Json<serde_json::Value>), AppError> {
    if let Some(cookie) = headers.get(http::header::COOKIE) {
        if let Ok(cookie_str) = cookie.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("session=") {
                    let token_hash = sha256_hash(token);
                    let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
                        .bind(&token_hash)
                        .execute(&state.pool)
                        .await;
                }
            }
        }
    }

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        http::header::SET_COOKIE,
        "session=; HttpOnly; Secure; Path=/; Max-Age=0; SameSite=Lax".parse().unwrap(),
    );

    Ok((resp_headers, axum::Json(serde_json::json!({"status": "ok"}))))
}
