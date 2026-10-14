mod agent_hub;
mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod state;

use std::net::SocketAddr;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{agent_hub::AgentHub, config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let db = db::connect(&config.database_url).await?;
    let hub = AgentHub::new();

    let state = AppState {
        db,
        config: config.clone(),
        hub,
    };

    let app = routes::app_router(state);
    let addr: SocketAddr = config.bind_addr.parse()?;
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
