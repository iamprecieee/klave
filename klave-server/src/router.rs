use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    handlers::{agents, events, health, orca, transactions},
    middleware::{api_key_auth, operator_key_auth},
    state::AppState,
};

pub fn build_router(state: AppState) -> Router {
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(100)
        .burst_size(100)
        .finish()
        .unwrap();

    let creation_governor_conf = GovernorConfigBuilder::default()
        .per_second(60)
        .burst_size(5)
        .finish()
        .unwrap();

    let public_routes = Router::new()
        .route("/agents", post(agents::create_agent))
        .layer(GovernorLayer::new(Arc::new(creation_governor_conf)));

    let operator_only_routes = Router::new()
        .route("/agents", get(agents::list_agents))
        .route("/agents/{id}", delete(agents::deactivate_agent))
        .route("/agents/{id}/policy", put(agents::update_policy))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            operator_key_auth,
        ));

    let shared_routes = Router::new()
        .route("/agents/{id}/history", get(agents::get_agent_history))
        .route("/agents/{id}/balance", get(agents::get_agent_balance))
        .route("/agents/{id}/tokens", get(agents::get_agent_token_balances))
        .route("/agents/{id}/notify", post(agents::notify_balance_updated))
        .route("/events", get(events::sse_handler))
        .layer(middleware::from_fn_with_state(state.clone(), api_key_auth));

    let agent_only_routes = Router::new()
        .route(
            "/agents/{id}/transactions",
            post(transactions::execute_transaction),
        )
        .route("/agents/{id}/orca/swap", post(orca::execute_swap))
        .route("/agents/{id}/orca/quote", post(orca::get_swap_quote))
        .route("/orca/pools", get(orca::get_orca_pools))
        .layer(middleware::from_fn_with_state(state.clone(), api_key_auth));

    let api_routes = Router::new()
        .merge(public_routes)
        .merge(operator_only_routes)
        .merge(shared_routes)
        .merge(agent_only_routes);

    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api/v1", api_routes)
        .layer(GovernorLayer::new(Arc::new(governor_conf)))
        .with_state(state)
}
