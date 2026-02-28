use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

use crate::{response::ApiResponse, state::AppState};

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub agent_id: Option<String>,
    pub is_operator: bool,
}

pub async fn api_key_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|val| val.to_str().ok());

    let (id_from_path, is_agent_path) = {
        let path = request.uri().path();
        let parts: Vec<&str> = path.split('/').collect(); // /api/v1/agents/{id}/...
        if parts.len() >= 5 && parts[1] == "api" && parts[2] == "v1" && parts[3] == "agents" {
            (Some(parts[4].to_string()), true)
        } else {
            (None, false)
        }
    };

    match api_key {
        Some(key) => {
            if key
                .as_bytes()
                .ct_eq(state.config.operator_api_key.as_bytes())
                .into()
            {
                request.extensions_mut().insert(AuthContext {
                    agent_id: None,
                    is_operator: true,
                });
                return next.run(request).await;
            }

            if let Ok(Some(agent)) = state.agent_repo.find_by_key_hash(key).await {
                if is_agent_path {
                    if let Some(ref path_id) = id_from_path {
                        if path_id != &agent.id {
                            return ApiResponse::<()>::error(
                                StatusCode::FORBIDDEN,
                                "Forbidden: Agent ID mismatch",
                            )
                            .into_response();
                        }
                    }
                }

                request.extensions_mut().insert(AuthContext {
                    agent_id: Some(agent.id),
                    is_operator: false,
                });
                return next.run(request).await;
            }

            ApiResponse::<()>::error(StatusCode::UNAUTHORIZED, "Invalid X-API-Key").into_response()
        }
        None => ApiResponse::<()>::error(StatusCode::UNAUTHORIZED, "Missing X-API-Key header")
            .into_response(),
    }
}

pub async fn operator_key_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|val| val.to_str().ok());

    match api_key {
        Some(key)
            if key
                .as_bytes()
                .ct_eq(state.config.operator_api_key.as_bytes())
                .into() =>
        {
            request.extensions_mut().insert(AuthContext {
                agent_id: None,
                is_operator: true,
            });
            next.run(request).await
        }
        _ => ApiResponse::<()>::error(
            StatusCode::UNAUTHORIZED,
            "missing or invalid X-API-Key header (operator key required)",
        )
        .into_response(),
    }
}
