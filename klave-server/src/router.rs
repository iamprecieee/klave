use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post, put};

use crate::handlers::{agents, health, orca, transactions};
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
        .route("/agents/{id}/orca/swap", post(orca::execute_swap))
        .route("/agents/{id}/orca/open-position", post(orca::open_position))
        .route(
            "/agents/{id}/orca/position/increase",
            put(orca::increase_liquidity),
        )
        .route(
            "/agents/{id}/orca/position/decrease",
            put(orca::decrease_liquidity),
        )
        .route("/agents/{id}/orca/harvest", post(orca::harvest))
        .route(
            "/agents/{id}/orca/position/{position}",
            delete(orca::close_position),
        )
        .layer(middleware::from_fn_with_state(state.clone(), api_key_auth))
        .with_state(state);

    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api/v1", api_routes)
}
