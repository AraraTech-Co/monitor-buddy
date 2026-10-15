use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    error::AppResult,
    models::ActivityEntry,
    routes::auth::MasterUser,
    state::AppState,
    tracker,
};

#[derive(Debug, Deserialize)]
pub struct TrackerQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_activity(
    State(state): State<AppState>,
    _master: MasterUser,
    Query(q): Query<TrackerQuery>,
) -> AppResult<Json<Vec<ActivityEntry>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = tracker::list(&state.db, limit, offset).await?;
    Ok(Json(rows))
}
