use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post, put};

use crate::handlers::{agents, health, transactions};
use crate::middleware::api_key_auth;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/agents", post(agents::create_agent))
        .route("/agents", get(agents::list_agents))
        .route("/agents/{id}", delete(agents::deactivate_agent))
        .route("/agents/{id}/history", get(agents::get_agent_history))
        .route("/agents/{id}/balance", get(agents::get_agent_balance))
        .route("/agents/{id}/policy", put(agents::update_policy))
        .route(
            "/agents/{id}/transactions",
            post(transactions::execute_transaction),
        )
        .layer(middleware::from_fn_with_state(state.clone(), api_key_auth))
        .with_state(state);

    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api/v1", api_routes)
}
