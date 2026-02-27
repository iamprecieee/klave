use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{response::ApiResponse, state::AppState};

pub async fn api_key_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
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

pub async fn operator_key_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if key == state.config.operator_api_key => next.run(request).await,
        _ => {
            let response = ApiResponse::<()>::error(
                StatusCode::UNAUTHORIZED,
                "missing or invalid X-API-Key header (operator key required)",
            );
            IntoResponse::into_response(response)
        }
    }
}

pub async fn any_key_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if key == state.config.api_key || key == state.config.operator_api_key => {
            next.run(request).await
        }
        _ => {
            let response = ApiResponse::<()>::error(
                StatusCode::UNAUTHORIZED,
                "missing or invalid X-API-Key header",
            );
            IntoResponse::into_response(response)
        }
    }
}
