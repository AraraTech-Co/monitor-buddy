use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::Utc;
use shared::{AgentToServer, ContainerInfo, ServerToAgent};
use tokio::sync::{Mutex, RwLock, oneshot};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HostRuntime {
    pub containers: Vec<ContainerInfo>,
    pub last_heartbeat: chrono::DateTime<Utc>,
}

struct AgentConn {
    conn_id: Uuid,
    tx: tokio::sync::mpsc::UnboundedSender<ServerToAgent>,
}

pub struct AgentHub {
    agents: Mutex<HashMap<String, AgentConn>>,
    runtime: RwLock<HashMap<String, HostRuntime>>,
    pending: Mutex<HashMap<String, oneshot::Sender<AgentToServer>>>,
    log_pending: Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<AgentToServer>>>,
}

impl AgentHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            agents: Mutex::new(HashMap::new()),
            runtime: RwLock::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            log_pending: Mutex::new(HashMap::new()),
        })
    }

    pub async fn is_online(&self, host_id: &str) -> bool {
        self.agents.lock().await.contains_key(host_id)
    }

    pub async fn runtime(&self, host_id: &str) -> Option<HostRuntime> {
        self.runtime.read().await.get(host_id).cloned()
    }

    pub async fn all_runtimes(&self) -> HashMap<String, HostRuntime> {
        self.runtime.read().await.clone()
    }

    pub async fn register(
        &self,
        host_id: String,
        tx: tokio::sync::mpsc::UnboundedSender<ServerToAgent>,
    ) -> Uuid {
        let conn_id = Uuid::new_v4();
        let replaced = self
            .agents
            .lock()
            .await
            .insert(host_id.clone(), AgentConn { conn_id, tx })
            .is_some();
        tracing::info!(%host_id, %conn_id, replaced, "agent registered");
        conn_id
    }

    pub async fn disconnect(&self, host_id: &str) {
        self.agents.lock().await.remove(host_id);
        tracing::info!(host_id, "agent disconnected");
    }

    /// Remove the host only if the closed connection is the current one,
    /// so a stale connection can't take down a newer one.
    pub async fn disconnect_conn(&self, host_id: &str, conn_id: Uuid) {
        let mut agents = self.agents.lock().await;
        let is_current = agents
            .get(host_id)
            .map(|current| current.conn_id == conn_id)
            .unwrap_or(false);
        if is_current {
            agents.remove(host_id);
            tracing::info!(host_id, %conn_id, "agent disconnected");
        } else {
            tracing::info!(host_id, %conn_id, "stale agent connection closed; current connection kept");
        }
    }

    pub async fn update_heartbeat(&self, host_id: &str, containers: Vec<ContainerInfo>) {
        self.runtime.write().await.insert(
            host_id.to_string(),
            HostRuntime {
                containers,
                last_heartbeat: Utc::now(),
            },
        );
    }

    pub async fn dispatch_agent_message(&self, host_id: &str, msg: AgentToServer) {
        match &msg {
            AgentToServer::Heartbeat { containers, .. } => {
                self.update_heartbeat(host_id, containers.clone()).await;
            }
            AgentToServer::Containers { request_id, .. }
            | AgentToServer::RestartResult { request_id, .. } => {
                if let Some(tx) = self.pending.lock().await.remove(request_id) {
                    let _ = tx.send(msg);
                }
            }
            AgentToServer::LogsChunk {
                request_id, done, ..
            } => {
                let request_id = request_id.clone();
                let finished = *done;
                let mut logs = self.log_pending.lock().await;
                if let Some(tx) = logs.get(&request_id) {
                    let _ = tx.send(msg);
                    if finished {
                        logs.remove(&request_id);
                    }
                }
            }
            AgentToServer::Error {
                request_id: Some(request_id),
                ..
            } => {
                if let Some(tx) = self.pending.lock().await.remove(request_id) {
                    let _ = tx.send(msg.clone());
                }
                if let Some(tx) = self.log_pending.lock().await.remove(request_id) {
                    let _ = tx.send(msg);
                }
            }
            _ => {}
        }
    }

    async fn send(&self, host_id: &str, msg: ServerToAgent) -> AppResult<()> {
        let agents = self.agents.lock().await;
        let agent = agents.get(host_id).ok_or(AppError::AgentOffline)?;
        agent.tx.send(msg).map_err(|_| AppError::AgentOffline)
    }

    async fn request(
        &self,
        host_id: &str,
        request_id: String,
        msg: ServerToAgent,
        timeout: Duration,
    ) -> AppResult<AgentToServer> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id.clone(), tx);
        if let Err(e) = self.send(host_id, msg).await {
            self.pending.lock().await.remove(&request_id);
            return Err(e);
        }
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&request_id);
                Err(AppError::AgentOffline)
            }
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                Err(AppError::BadRequest("agent request timed out".into()))
            }
        }
    }

    pub async fn list_containers(&self, host_id: &str) -> AppResult<Vec<ContainerInfo>> {
        if let Some(rt) = self.runtime(host_id).await {
            if self.is_online(host_id).await {
                return Ok(rt.containers);
            }
        }
        let request_id = Uuid::new_v4().to_string();
        let msg = ServerToAgent::ListContainers {
            request_id: request_id.clone(),
        };
        match self
            .request(host_id, request_id, msg, Duration::from_secs(15))
            .await?
        {
            AgentToServer::Containers { containers, .. } => Ok(containers),
            AgentToServer::Error { message, .. } => Err(AppError::BadRequest(message)),
            other => Err(AppError::internal(format!("unexpected: {other:?}"))),
        }
    }

    pub async fn restart_container(&self, host_id: &str, container_id: &str) -> AppResult<()> {
        let request_id = Uuid::new_v4().to_string();
        let msg = ServerToAgent::Restart {
            request_id: request_id.clone(),
            container_id: container_id.to_string(),
        };
        match self
            .request(host_id, request_id, msg, Duration::from_secs(60))
            .await?
        {
            AgentToServer::RestartResult { ok, error, .. } => {
                if ok {
                    Ok(())
                } else {
                    Err(AppError::BadRequest(
                        error.unwrap_or_else(|| "restart failed".into()),
                    ))
                }
            }
            AgentToServer::Error { message, .. } => Err(AppError::BadRequest(message)),
            other => Err(AppError::internal(format!("unexpected: {other:?}"))),
        }
    }

    pub async fn get_logs(
        &self,
        host_id: &str,
        container_id: &str,
        tail: u32,
    ) -> AppResult<String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        self.log_pending
            .lock()
            .await
            .insert(request_id.clone(), tx);

        let msg = ServerToAgent::GetLogs {
            request_id: request_id.clone(),
            container_id: container_id.to_string(),
            tail,
        };

        if let Err(e) = self.send(host_id, msg).await {
            self.log_pending.lock().await.remove(&request_id);
            return Err(e);
        }

        let mut out = String::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                self.log_pending.lock().await.remove(&request_id);
                return Err(AppError::BadRequest("log request timed out".into()));
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(AgentToServer::LogsChunk { data, done, .. })) => {
                    out.push_str(&data);
                    if done {
                        break;
                    }
                }
                Ok(Some(AgentToServer::Error { message, .. })) => {
                    return Err(AppError::BadRequest(message));
                }
                Ok(Some(_)) => continue,
                Ok(None) => return Err(AppError::AgentOffline),
                Err(_) => {
                    self.log_pending.lock().await.remove(&request_id);
                    return Err(AppError::BadRequest("log request timed out".into()));
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Agent reconnects while the old connection is still open; when the old
    // one closes, the host must stay online.
    #[tokio::test]
    async fn stale_connection_does_not_drop_current() {
        let hub = AgentHub::new();
        let (tx_old, _rx_old) = tokio::sync::mpsc::unbounded_channel();
        let (tx_new, _rx_new) = tokio::sync::mpsc::unbounded_channel();

        let old = hub.register("host-1".into(), tx_old).await;
        let new = hub.register("host-1".into(), tx_new).await;

        hub.disconnect_conn("host-1", old).await;
        assert!(hub.is_online("host-1").await, "stale connection removed the host");

        hub.disconnect_conn("host-1", new).await;
        assert!(!hub.is_online("host-1").await, "current connection closed but host stayed online");
    }
}
