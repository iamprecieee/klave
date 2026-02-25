use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{response::ApiResponse, state::AppState};

pub async fn api_key_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
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
            IntoResponse::into_response(response)
        }
    }
}
