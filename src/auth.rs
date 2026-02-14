use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

#[allow(dead_code)]
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request
        .extensions()
        .get::<Option<String>>()
        .cloned()
        .flatten();

    let Some(expected_token) = token else {
        // No auth configured → pass through
        let mut response = next.run(request).await;
        inject_request_id(&mut response);
        return Ok(response);
    };

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(h) if h.strip_prefix("Bearer ").unwrap_or(h) == expected_token => {
            let mut response = next.run(request).await;
            inject_request_id(&mut response);
            Ok(response)
        }
        _ => {
            tracing::warn!(
                "Auth failed from {}",
                request
                    .headers()
                    .get("x-forwarded-for")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn inject_request_id(response: &mut Response) {
    let id = Uuid::new_v4().to_string();
    response.headers_mut().insert(
        "x-request-id",
        id.parse().unwrap(),
    );
}

/// Simple token check function for endpoints that check auth directly.
pub fn check_token(auth_header: Option<&str>, expected: &str) -> bool {
    match auth_header {
        Some(h) => h.strip_prefix("Bearer ").unwrap_or(h) == expected,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_token_bearer() {
        assert!(check_token(Some("Bearer secret123"), "secret123"));
    }

    #[test]
    fn test_check_token_raw() {
        assert!(check_token(Some("secret123"), "secret123"));
    }

    #[test]
    fn test_check_token_wrong() {
        assert!(!check_token(Some("Bearer wrong"), "secret123"));
    }

    #[test]
    fn test_check_token_missing() {
        assert!(!check_token(None, "secret123"));
    }
}
