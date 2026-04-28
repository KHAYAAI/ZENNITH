use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::StatusCode,
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub role: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

fn get_jwt_secret() -> Vec<u8> {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "zenith-platform-secret-key-change-in-production".to_string())
        .into_bytes()
}

pub fn generate_token(sub: &str, role: &str) -> Result<String, jsonwebtoken::errors::Error> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: sub.to_string(),
        exp: now + 86400 * 30, // 30 days
        iat: now,
        role: role.to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&get_jwt_secret()),
    )
}

pub fn verify_token(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&get_jwt_secret()),
        &Validation::default(),
    )
}

pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for health check and metrics
    let path = request.uri().path();
    if path == "/health" || path == "/v1/metrics" {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token from Authorization header
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                match verify_token(token) {
                    Ok(token_data) => {
                        request.extensions_mut().insert(token_data.claims);
                        return Ok(next.run(request).await);
                    }
                    Err(e) => {
                        warn!("Token verification failed: {}", e);
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
            }
        }
    }

    // If no valid token, allow with anonymous role for read-only operations
    if request.method() == axum::http::Method::GET {
        let claims = Claims {
            sub: "anonymous".to_string(),
            exp: 0,
            iat: 0,
            role: "viewer".to_string(),
        };
        request.extensions_mut().insert(claims);
        Ok(next.run(request).await)
    } else {
        // Write operations require authentication
        warn!("Write operation without authentication");
        Err(StatusCode::UNAUTHORIZED)
    }
}
