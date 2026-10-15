use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Utc;
use sqlx::SqlitePool;

use crate::{agent_hub::AgentHub, error::AppResult};

const TICK: Duration = Duration::from_secs(15);
const RESTART_COOLDOWN: Duration = Duration::from_secs(90);

pub async fn run_loop(db: SqlitePool, hub: Arc<AgentHub>) {
    let mut last_restart: HashMap<(String, String), Instant> = HashMap::new();
    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        if let Err(e) = tick(&db, &hub, &mut last_restart).await {
            tracing::warn!("overwatch tick failed: {e}");
        }
    }
}

async fn tick(
    db: &SqlitePool,
    hub: &AgentHub,
    last_restart: &mut HashMap<(String, String), Instant>,
) -> AppResult<()> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT host_id, container_id FROM overwatch WHERE enabled = 1",
    )
    .fetch_all(db)
    .await?;

    for (host_id, container_id) in rows {
        if !hub.is_online(&host_id).await {
            continue;
        }

        let Some(rt) = hub.runtime(&host_id).await else {
            continue;
        };

        let Some(container) = rt
            .containers
            .iter()
            .find(|c| c.id == container_id || c.id.starts_with(&container_id) || container_id.starts_with(&c.id))
        else {
            continue;
        };

        let Some(health) = container.health.as_deref() else {
            // Healthcheck disappeared — disable overwatch
            tracing::warn!(
                %host_id,
                container = %container.name,
                "overwatch disabled: container has no healthcheck"
            );
            let _ = sqlx::query(
                "UPDATE overwatch SET enabled = 0, updated_at = ? WHERE host_id = ? AND container_id = ?",
            )
            .bind(Utc::now())
            .bind(&host_id)
            .bind(&container_id)
            .execute(db)
            .await;
            continue;
        };

        if !health.eq_ignore_ascii_case("unhealthy") {
            continue;
        }

        let key = (host_id.clone(), container_id.clone());
        if let Some(at) = last_restart.get(&key) {
            if at.elapsed() < RESTART_COOLDOWN {
                continue;
            }
        }

        tracing::info!(
            %host_id,
            container = %container.name,
            %health,
            "overwatch restarting unhealthy container"
        );

        match hub.restart_container(&host_id, &container.id).await {
            Ok(()) => {
                last_restart.insert(key, Instant::now());
            }
            Err(e) => {
                tracing::warn!(
                    %host_id,
                    container = %container.name,
                    "overwatch restart failed: {e}"
                );
            }
        }
    }

    Ok(())
}
