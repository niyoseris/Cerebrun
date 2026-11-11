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
pub struct AuthStartParams {
    pub next: Option<String>,
}

/// Only allow same-origin relative paths. This prevents the `next` parameter
/// from becoming an open redirect.
fn safe_next_path(value: &str) -> Option<&str> {
    if !value.starts_with('/') || value.starts_with("//") || value.len() > 512 {
        return None;
    }
    if value.chars().any(|c| {
        c.is_control() || c.is_whitespace() || c == ';' || c == '\\' || c == '"' || c == ','
    }) {
        return None;
    }
    Some(value)
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
    // Prefer the configured canonical callback URL. Deriving it from the Host
    // header makes the OAuth redirect depend on whether the user arrived on
    // `www.` or the apex domain, which must both be registered in Google Cloud.
    if !config.google_redirect_uri.is_empty() {
        return config.google_redirect_uri.clone();
    }

    if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
        let final_scheme = if host.contains("localhost") || host.contains("127.0.0.1") {
            "http"
        } else {
            "https"
        };
        format!("{}://{}/auth/google/callback", final_scheme, host)
    } else {
        "http://localhost:5000/auth/google/callback".to_string()
    }
}

pub async fn google_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AuthStartParams>,
) -> Result<(HeaderMap, Redirect), AppError> {
    let csrf_state = generate_random_key("state");

    let redirect_uri = get_redirect_uri(&headers, &state.config);
    tracing::info!("Google Auth Start - Redirect URI: {}", redirect_uri);

    let secure = if redirect_uri.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };

    let mut resp_headers = HeaderMap::new();
    let cookie_value = format!(
        "oauth_state={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax{}",
        csrf_state, secure
    );
    resp_headers.insert(http::header::SET_COOKIE, cookie_value.parse().unwrap());

    let redirect_cookie = format!(
        "oauth_redirect_uri={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax{}",
        redirect_uri, secure
    );
    resp_headers.append(http::header::SET_COOKIE, redirect_cookie.parse().unwrap());

    if let Some(next) = params.next.as_deref().and_then(safe_next_path) {
        let next_cookie = format!(
            "oauth_next={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax{}",
            next, secure
        );
        resp_headers.append(http::header::SET_COOKIE, next_cookie.parse().unwrap());
    }

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
    let callback_state = params
        .state
        .as_deref()
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
        return Err(AppError::OAuth(
            "OAuth state mismatch - possible CSRF attack".to_string(),
        ));
    }

    let redirect_uri = cookie_header
        .split(';')
        .filter_map(|s| s.trim().strip_prefix("oauth_redirect_uri="))
        .next()
        .map(|s| s.to_string())
        .unwrap_or_else(|| get_redirect_uri(&headers, &state.config));

    tracing::info!("Google Callback - Redirect URI: {}", redirect_uri);
    tracing::info!("Google Callback - State: {:?}", params.state);

    let token_http = reqwest::Client::new()
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
            AppError::OAuth("Token request failed".to_string())
        })?;

    let token_status = token_http.status();
    let token_body = token_http.text().await.map_err(|e| {
        tracing::error!("OAuth - Failed to read token response: {}", e);
        AppError::OAuth("Token response could not be read".to_string())
    })?;

    if !token_status.is_success() {
        tracing::error!(
            "OAuth - Token exchange rejected ({}): {}",
            token_status.as_u16(),
            token_body
        );
        return Err(AppError::OAuth(format!(
            "Token exchange failed with status {}",
            token_status.as_u16()
        )));
    }

    let token_response: GoogleTokenResponse = serde_json::from_str(&token_body).map_err(|e| {
        tracing::error!(
            "OAuth - Token JSON parsing failed: {} body={}",
            e,
            token_body
        );
        AppError::OAuth("Token response could not be parsed".to_string())
    })?;

    tracing::info!("OAuth - Token Received Successfully");

    let userinfo_http = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token_response.access_token)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("OAuth - Failed to fetch userinfo: {}", e);
            AppError::OAuth("User info request failed".to_string())
        })?;

    let userinfo_status = userinfo_http.status();
    let userinfo_body = userinfo_http.text().await.map_err(|e| {
        tracing::error!("OAuth - Failed to read userinfo response: {}", e);
        AppError::OAuth("User info response could not be read".to_string())
    })?;

    if !userinfo_status.is_success() {
        tracing::error!(
            "OAuth - Userinfo rejected ({}): {}",
            userinfo_status.as_u16(),
            userinfo_body
        );
        return Err(AppError::OAuth(format!(
            "User info request failed with status {}",
            userinfo_status.as_u16()
        )));
    }

    let user_info: GoogleUserInfo = serde_json::from_str(&userinfo_body).map_err(|e| {
        tracing::error!(
            "OAuth - Failed to parse userinfo JSON: {} body={}",
            e,
            userinfo_body
        );
        AppError::OAuth("User info response could not be parsed".to_string())
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
    resp_headers.insert(http::header::SET_COOKIE, session_cookie.parse().unwrap());
    resp_headers.append(
        http::header::SET_COOKIE,
        "oauth_state=; HttpOnly; Path=/; Max-Age=0".parse().unwrap(),
    );
    resp_headers.append(
        http::header::SET_COOKIE,
        "oauth_redirect_uri=; HttpOnly; Path=/; Max-Age=0"
            .parse()
            .unwrap(),
    );
    resp_headers.append(
        http::header::SET_COOKIE,
        "oauth_next=; HttpOnly; Path=/; Max-Age=0".parse().unwrap(),
    );

    let next_path = cookie_header
        .split(';')
        .filter_map(|s| s.trim().strip_prefix("oauth_next="))
        .next()
        .and_then(safe_next_path)
        .unwrap_or("/");

    Ok((resp_headers, Redirect::temporary(next_path)))
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
        "session=; HttpOnly; Secure; Path=/; Max-Age=0; SameSite=Lax"
            .parse()
            .unwrap(),
    );

    Ok((
        resp_headers,
        axum::Json(serde_json::json!({"status": "ok"})),
    ))
}

#[cfg(test)]
mod tests {
    use super::{get_redirect_uri, safe_next_path};
    use crate::config::AppConfig;
    use axum::http::{HeaderMap, HeaderValue};

    fn config(redirect_uri: &str) -> AppConfig {
        AppConfig {
            database_url: "postgres://unused".to_string(),
            google_client_id: "id".to_string(),
            google_client_secret: "secret".to_string(),
            google_redirect_uri: redirect_uri.to_string(),
            session_secret: "secret".to_string(),
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        }
    }

    #[test]
    fn configured_redirect_uri_wins_over_host_header() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("www.cereb.run"));

        let uri = get_redirect_uri(&headers, &config("https://cereb.run/auth/google/callback"));
        assert_eq!(uri, "https://cereb.run/auth/google/callback");
    }

    #[test]
    fn host_header_is_used_when_no_redirect_uri_is_configured() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("localhost:5050"));

        let uri = get_redirect_uri(&headers, &config(""));
        assert_eq!(uri, "http://localhost:5050/auth/google/callback");
    }

    #[test]
    fn next_path_accepts_same_origin_paths() {
        assert_eq!(safe_next_path("/setup"), Some("/setup"));
        assert_eq!(safe_next_path("/setup?next=1"), Some("/setup?next=1"));
    }

    #[test]
    fn next_path_rejects_open_redirects() {
        assert_eq!(safe_next_path("//evil.example"), None);
        assert_eq!(safe_next_path("https://evil.example"), None);
        assert_eq!(safe_next_path("/a;b"), None);
        assert_eq!(safe_next_path("/a b"), None);
        assert_eq!(safe_next_path("/a\\b"), None);
    }
}
