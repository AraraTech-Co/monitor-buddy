use anyhow::{Context, Result};
use bollard::{
    Docker,
    container::{ListContainersOptions, LogsOptions, RestartContainerOptions},
};
use futures_util::StreamExt;
use shared::ContainerInfo;

#[derive(Clone)]
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .context("connect to local docker (is DOCKER_HOST / socket available?)")?;
        Ok(Self { docker })
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let opts = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        let list = self
            .docker
            .list_containers(Some(opts))
            .await
            .context("list containers")?;

        let mut out = Vec::with_capacity(list.len());
        for c in list {
            let id = c.id.unwrap_or_default();
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| id.chars().take(12).collect());
            let image = c.image.unwrap_or_else(|| "unknown".into());
            let status = c.status.clone().unwrap_or_default();
            let state = c.state.unwrap_or_default();

            // Health from Status field sometimes embeds health; also inspect when needed.
            let health = c
                .status
                .as_ref()
                .and_then(|s| {
                    if s.contains("(healthy)") {
                        Some("healthy".into())
                    } else if s.contains("(unhealthy)") {
                        Some("unhealthy".into())
                    } else if s.contains("(starting)") {
                        Some("starting".into())
                    } else {
                        None
                    }
                });

            // Prefer inspect health when available for running containers
            let health = if health.is_none() && state.eq_ignore_ascii_case("running") {
                match self.docker.inspect_container(&id, None).await {
                    Ok(insp) => insp
                        .state
                        .and_then(|st| st.health)
                        .and_then(|h| h.status)
                        .map(|s| format!("{s}").to_lowercase()),
                    Err(_) => None,
                }
            } else {
                health
            };

            out.push(ContainerInfo {
                id,
                name,
                image,
                status,
                state,
                health,
            });
        }

        // Sort by name for stable UI
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub async fn logs(&self, container_id: &str, tail: u32) -> Result<String> {
        let opts = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            timestamps: false,
            follow: false,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(opts));
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            let chunk = item.context("read log chunk")?;
            out.push_str(&chunk.to_string());
        }
        Ok(out)
    }

    pub async fn restart(&self, container_id: &str) -> Result<()> {
        let opts = RestartContainerOptions { t: 10 };
        self.docker
            .restart_container(container_id, Some(opts))
            .await
            .with_context(|| format!("restart {container_id}"))?;
        Ok(())
    }
}
