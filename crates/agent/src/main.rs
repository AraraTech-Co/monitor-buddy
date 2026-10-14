mod docker;

use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use shared::{AgentToServer, ServerToAgent};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::docker::DockerClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "ws://localhost:8080/agent/ws".into());
    let token = std::env::var("AGENT_TOKEN").context("AGENT_TOKEN is required")?;
    let heartbeat_secs: u64 = std::env::var("HEARTBEAT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    let docker = DockerClient::new()?;

    loop {
        match run_session(&server_url, &token, &docker, heartbeat_secs).await {
            Ok(()) => tracing::warn!("session ended cleanly; reconnecting"),
            Err(e) => tracing::error!("session error: {e:#}; reconnecting in 5s"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_session(
    server_url: &str,
    token: &str,
    docker: &DockerClient,
    heartbeat_secs: u64,
) -> Result<()> {
    tracing::info!("connecting to {server_url}");
    let (ws, _) = connect_async(server_url)
        .await
        .with_context(|| format!("connect to {server_url}"))?;
    let (mut sink, mut stream) = ws.split();

    let auth = AgentToServer::Auth {
        token: token.to_string(),
    };
    sink.send(Message::Text(serde_json::to_string(&auth)?.into()))
        .await?;

    // Wait for auth response
    let auth_msg = wait_text(&mut stream).await?;
    match serde_json::from_str::<ServerToAgent>(&auth_msg)? {
        ServerToAgent::AuthOk { host_id, host_name } => {
            tracing::info!(%host_id, %host_name, "authenticated");
        }
        ServerToAgent::AuthFail { reason } => bail!("auth failed: {reason}"),
        other => bail!("unexpected auth response: {other:?}"),
    }

    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<AgentToServer>();

    // Heartbeat task
    let hb_docker = docker.clone();
    let hb_tx = out_tx.clone();
    let heartbeat = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(heartbeat_secs));
        loop {
            interval.tick().await;
            match hb_docker.list_containers().await {
                Ok(containers) => {
                    let _ = hb_tx.send(AgentToServer::Heartbeat {
                        containers,
                        at: chrono::Utc::now(),
                    });
                }
                Err(e) => tracing::error!("heartbeat list failed: {e:#}"),
            }
        }
    });

    // Writer task
    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(text) => {
                    if sink.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("serialize: {e}");
                    break;
                }
            }
        }
    });

    // Send initial heartbeat immediately
    if let Ok(containers) = docker.list_containers().await {
        let _ = out_tx.send(AgentToServer::Heartbeat {
            containers,
            at: chrono::Utc::now(),
        });
    }

    while let Some(msg) = stream.next().await {
        let msg = msg.context("ws read")?;
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
            Message::Binary(_) | Message::Frame(_) => continue,
        };

        let cmd: ServerToAgent = match serde_json::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("bad server message: {e}");
                continue;
            }
        };

        match cmd {
            ServerToAgent::ListContainers { request_id } => {
                let resp = match docker.list_containers().await {
                    Ok(containers) => AgentToServer::Containers {
                        request_id,
                        containers,
                    },
                    Err(e) => AgentToServer::Error {
                        request_id: Some(request_id),
                        message: e.to_string(),
                    },
                };
                let _ = out_tx.send(resp);
            }
            ServerToAgent::GetLogs {
                request_id,
                container_id,
                tail,
            } => {
                match docker.logs(&container_id, tail).await {
                    Ok(data) => {
                        let _ = out_tx.send(AgentToServer::LogsChunk {
                            request_id,
                            data,
                            done: true,
                        });
                    }
                    Err(e) => {
                        let _ = out_tx.send(AgentToServer::Error {
                            request_id: Some(request_id),
                            message: e.to_string(),
                        });
                    }
                }
            }
            ServerToAgent::Restart {
                request_id,
                container_id,
            } => {
                let resp = match docker.restart(&container_id).await {
                    Ok(()) => AgentToServer::RestartResult {
                        request_id,
                        ok: true,
                        error: None,
                    },
                    Err(e) => AgentToServer::RestartResult {
                        request_id,
                        ok: false,
                        error: Some(e.to_string()),
                    },
                };
                let _ = out_tx.send(resp);
            }
            ServerToAgent::AuthOk { .. } | ServerToAgent::AuthFail { .. } => {}
        }
    }

    heartbeat.abort();
    writer.abort();
    Ok(())
}

async fn wait_text<S>(stream: &mut S) -> Result<String>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(msg) = stream.next().await {
        let msg = msg.context("ws read during auth")?;
        match msg {
            Message::Text(t) => return Ok(t.to_string()),
            Message::Close(_) => bail!("connection closed during auth"),
            _ => continue,
        }
    }
    bail!("connection closed during auth")
}
