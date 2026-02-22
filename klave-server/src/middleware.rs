use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::response::ApiResponse;
use crate::state::AppState;

pub async fn api_key_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();

    let requires_auth = matches!(method, Method::POST | Method::PUT | Method::DELETE);

    if !requires_auth {
        return next.run(request).await;
    }

    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if key == state.config.api_key => next.run(request).await,
        _ => {
            let response = ApiResponse::<()>::error(
                StatusCode::UNAUTHORIZED,
                "missing or invalid X-API-Key header",
            );
            axum::response::IntoResponse::into_response(response)
        }
    }
}
