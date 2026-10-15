pub mod agent_ws;
pub mod auth;
pub mod hosts;
pub mod tracker_api;
pub mod users;

use std::path::PathBuf;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
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
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::COOKIE,
        ]);

    let api = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/users", get(users::list_users).post(users::create_user))
        .route("/api/users/{id}", delete(users::delete_user))
        .route("/api/tracker", get(tracker_api::list_activity))
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
        .route(
            "/api/hosts/{id}/containers/{cid}/overwatch",
            put(hosts::set_overwatch),
        )
        .route("/agent/ws", get(agent_ws::agent_ws))
        .with_state(state.clone());

    let static_dir = PathBuf::from(&state.config.static_dir);
    let index = static_dir.join("index.html");

    let router = if static_dir.is_dir() && index.is_file() {
        tracing::info!(dir = %static_dir.display(), "serving frontend static files");
        let spa = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index));
        api.fallback_service(spa)
    } else {
        tracing::warn!(
            dir = %static_dir.display(),
            "STATIC_DIR missing or empty; API-only mode (no SPA)"
        );
        api
    };

    router.layer(cors).layer(TraceLayer::new_for_http())
}
