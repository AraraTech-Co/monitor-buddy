use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{agent_hub::AgentHub, config::Config};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
    pub hub: Arc<AgentHub>,
}
