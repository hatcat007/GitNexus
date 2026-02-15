use axum::{
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    Json,
};
use serde_json::json;

pub fn verify_bearer(
    headers: &HeaderMap,
    expected_key: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let token = extract_bearer_token(headers)?;

    if token.trim() != expected_key {
        return Err(unauthorized("Invalid API key"));
    }

    Ok(())
}

pub fn extract_bearer_token(
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let Some(raw_header) = headers.get(AUTHORIZATION) else {
        return Err(unauthorized("Missing Authorization header"));
    };

    let Ok(value) = raw_header.to_str() else {
        return Err(unauthorized("Invalid Authorization header"));
    };

    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(unauthorized("Authorization must use Bearer token"));
    };

    Ok(token.to_string())
}

fn unauthorized(message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": {
                "code": "UNAUTHORIZED",
                "message": message
            }
        })),
    )
}
