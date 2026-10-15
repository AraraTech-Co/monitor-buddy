use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use serde::Deserialize;
use shared::ContainerInfo;
use uuid::Uuid;

use crate::{
    auth::{generate_agent_token, hash_token},
    error::{AppError, AppResult},
    models::{CreateHostRequest, CreateHostResponse, Host, HostResponse},
    routes::auth::AuthUser,
    state::AppState,
};

fn unhealthy_count(containers: &[ContainerInfo]) -> usize {
    containers
        .iter()
        .filter(|c| {
            let health_bad = c
                .health
                .as_deref()
                .map(|h| h.eq_ignore_ascii_case("unhealthy"))
                .unwrap_or(false);
            let state_bad = !c.state.eq_ignore_ascii_case("running");
            health_bad || state_bad
        })
        .count()
}

pub async fn list_hosts(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<Vec<HostResponse>>> {
    let hosts = sqlx::query_as::<_, Host>(
        "SELECT id, name, agent_token_hash, last_seen_at, created_at FROM hosts ORDER BY name",
    )
    .fetch_all(&state.db)
    .await?;

    let runtimes = state.hub.all_runtimes().await;
    let mut out = Vec::with_capacity(hosts.len());

    for host in hosts {
        let online = state.hub.is_online(&host.id).await;
        let (container_count, unhealthy) = if let Some(rt) = runtimes.get(&host.id) {
            (rt.containers.len(), unhealthy_count(&rt.containers))
        } else {
            (0, 0)
        };

        out.push(HostResponse {
            id: host.id,
            name: host.name,
            online,
            last_seen_at: host.last_seen_at,
            container_count,
            unhealthy_count: unhealthy,
            created_at: host.created_at,
        });
    }

    Ok(Json(out))
}

pub async fn create_host(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(body): Json<CreateHostRequest>,
) -> AppResult<Json<CreateHostResponse>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let id = Uuid::new_v4().to_string();
    let token = generate_agent_token();
    let token_hash = hash_token(&token);
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO hosts (id, name, agent_token_hash, last_seen_at, created_at) VALUES (?, ?, ?, NULL, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&token_hash)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db) = &e {
            if db.message().contains("UNIQUE") {
                return AppError::Conflict("host name already exists".into());
            }
        }
        AppError::from(e)
    })?;

    let docker_run = format!(
        "docker run -d --name mb-agent-{slug} --restart unless-stopped \
-v /var/run/docker.sock:/var/run/docker.sock \
-e SERVER_URL={ws} \
-e AGENT_TOKEN={token} \
{image}",
        slug = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .to_lowercase(),
        ws = state.config.public_ws_url,
        token = token,
        image = state.config.agent_image,
    );

    Ok(Json(CreateHostResponse {
        id,
        name,
        agent_token: token,
        docker_run,
    }))
}

pub async fn delete_host(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let res = sqlx::query("DELETE FROM hosts WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    state.hub.disconnect(&id).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn list_containers(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<ContainerInfo>>> {
    ensure_host_exists(&state, &id).await?;
    let containers = state.hub.list_containers(&id).await?;
    Ok(Json(containers))
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub tail: Option<u32>,
}

pub async fn get_logs(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, cid)): Path<(String, String)>,
    Query(q): Query<LogsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_host_exists(&state, &id).await?;
    let tail = q.tail.unwrap_or(200).clamp(1, 5000);
    let logs = state.hub.get_logs(&id, &cid, tail).await?;
    Ok(Json(serde_json::json!({ "logs": logs })))
}

pub async fn restart_container(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, cid)): Path<(String, String)>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_host_exists(&state, &id).await?;
    state.hub.restart_container(&id, &cid).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn ensure_host_exists(state: &AppState, id: &str) -> AppResult<()> {
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM hosts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }
    Ok(())
}
