use axum::{
    http::{StatusCode, header, Request},
    response::{IntoResponse, Redirect},
    extract::{Query, State},
    body::Body,
    middleware::Next,
};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use time::{Duration, OffsetDateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Config;

// Constants for Google OAuth
const GOOGLE_ISSUER: &str = "https://accounts.google.com";
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub azp: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

// Application state for auth
#[derive(Clone)]
pub struct AuthState {
    pub client: reqwest::Client,
    pub config: Config,
}

/// Google OAuth login handler
pub async fn login(
    State(state): State<AuthState>,
    Query(params): Query<HashMap<String, String>>
) -> impl IntoResponse {
    let return_to = params.get("return_to").map(|s| s.as_str()).unwrap_or("/C/RES/001");
    let state_param = urlencoding::encode(return_to);

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        GOOGLE_AUTH_URL,
        state.config.google_oauth.client_id,
        urlencoding::encode(&state.config.google_oauth.redirect_uri),
        urlencoding::encode("openid email profile"),
        state_param
    );

    Redirect::temporary(&auth_url)
}

/// Google OAuth callback handler
pub async fn callback(
    State(state): State<AuthState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    println!("Callback called: {:?}", params);

    let code = params.get("code").map(|s| s.as_str()).unwrap_or("");
    let return_to = params.get("state").map(|s| s.as_str()).unwrap_or("/C/RES/001");

    if code.is_empty() {
        println!("No code provided");
        return (StatusCode::BAD_REQUEST, "Missing code").into_response();
    }

    let token_resp = match state.client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &state.config.google_oauth.client_id),
            ("client_secret", &state.config.google_oauth.client_secret),
            ("redirect_uri", &state.config.google_oauth.redirect_uri),
        ])
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                println!("Token exchange failed: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Token exchange failed").into_response();
            }
        };

    let token_data: serde_json::Value = match token_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            println!("Invalid token response: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid token response").into_response();
        }
    };

    let access_token = match token_data.get("access_token").and_then(|a| a.as_str()) {
        Some(t) => t,
        None => {
            println!("No access token in response");
            return (StatusCode::BAD_REQUEST, "No access token").into_response();
        }
    };

    let user_resp = match state.client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                println!("Failed to get user info: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user info").into_response();
            }
        };

    let user_info: serde_json::Value = match user_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            println!("Invalid user info: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid user info").into_response();
        }
    };

    let email = match user_info.get("email").and_then(|e| e.as_str()) {
        Some(e) if e.ends_with(&state.config.google_oauth.allowed_email_domain) => e,
        _ => {
            println!("Invalid email domain");
            return (StatusCode::FORBIDDEN, "Invalid email domain").into_response();
        }
    };

    println!("Login successful for: {}", email);

    let now = OffsetDateTime::now_utc();
    let claims = Claims {
        iss: GOOGLE_ISSUER.to_string(),
        sub: user_info.get("sub").and_then(|s| s.as_str()).unwrap_or("").to_string(),
        email: email.to_string(),
        email_verified: true,
        azp: "app".to_string(),
        aud: "app".to_string(),
        iat: now.unix_timestamp(),
        exp: (now + Duration::hours(state.config.jwt.expiry_hours)).unix_timestamp(),
        name: user_info.get("name").and_then(|n| n.as_str()).map(String::from),
        picture: user_info.get("picture").and_then(|p| p.as_str()).map(String::from),
        given_name: user_info.get("given_name").and_then(|g| g.as_str()).map(String::from),
        family_name: user_info.get("family_name").and_then(|f| f.as_str()).map(String::from),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt.secret.as_ref())
    ).unwrap();

    println!("Token created, redirecting to: {}", return_to);

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <script>
        document.cookie = "token={}; path=/; max-age=86400";
        window.location.href = "{}";
    </script>
</head>
<body>登录成功，正在跳转...</body>
</html>"#,
        token, return_to
    );

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()
}

/// Auth middleware
pub async fn auth_middleware(
    State(state): State<AuthState>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let path = req.uri().path();

    // Extract token from cookie
    let cookies = req.headers().get(header::COOKIE).and_then(|v| v.to_str().ok());
    println!("Auth middleware for path: {}, cookies: {:?}", path, cookies);

    let token = cookies.and_then(|cookies| {
        cookies.split(';')
            .find(|c| c.trim().starts_with("token="))
            .map(|c| c.trim().strip_prefix("token=").unwrap_or(""))
    });

    let token = match token {
        Some(t) if !t.is_empty() => {
            println!("Token found: {}...", &t[..20.min(t.len())]);
            t
        },
        _ => {
            println!("No token found, redirecting to login");
            let return_url = urlencoding::encode(path);
            return Err(Redirect::temporary(&format!("/login?return_to={}", return_url)));
        }
    };

    // Validate token
    let decoding_key = DecodingKey::from_secret(state.config.jwt.secret.as_ref());
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    validation.set_audience(&["app"]);

    let _token_data = match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(data) => {
            println!("Token valid for: {}", data.claims.email);
            data
        },
        Err(e) => {
            println!("Token validation failed: {:?}", e);
            let return_url = urlencoding::encode(path);
            return Err(Redirect::temporary(&format!("/login?return_to={}", return_url)));
        }
    };

    Ok(next.run(req).await)
}
