pub mod agent_ws;
pub mod auth;
pub mod hosts;

use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn app_router(state: AppState) -> Router {
    let origin = state.config.frontend_origin.clone();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(origin.parse().expect("FRONTEND_ORIGIN")))
        .allow_credentials(true)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::COOKIE,
        ]);

    Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/hosts", get(hosts::list_hosts).post(hosts::create_host))
        .route("/api/hosts/{id}", delete(hosts::delete_host))
        .route("/api/hosts/{id}/containers", get(hosts::list_containers))
        .route(
            "/api/hosts/{id}/containers/{cid}/logs",
            get(hosts::get_logs),
        )
        .route(
            "/api/hosts/{id}/containers/{cid}/restart",
            post(hosts::restart_container),
        )
        .route("/agent/ws", get(agent_ws::agent_ws))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
