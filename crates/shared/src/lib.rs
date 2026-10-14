use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub health: Option<String>,
}

/// Messages sent from agent → server over the WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToServer {
    Auth {
        token: String,
    },
    Heartbeat {
        containers: Vec<ContainerInfo>,
        #[serde(default = "Utc::now")]
        at: DateTime<Utc>,
    },
    Containers {
        request_id: String,
        containers: Vec<ContainerInfo>,
    },
    LogsChunk {
        request_id: String,
        data: String,
        done: bool,
    },
    RestartResult {
        request_id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        message: String,
    },
}

/// Messages sent from server → agent over the WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToAgent {
    AuthOk {
        host_id: String,
        host_name: String,
    },
    AuthFail {
        reason: String,
    },
    ListContainers {
        request_id: String,
    },
    GetLogs {
        request_id: String,
        container_id: String,
        tail: u32,
    },
    Restart {
        request_id: String,
        container_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostSummary {
    pub id: String,
    pub name: String,
    pub online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub container_count: usize,
    pub unhealthy_count: usize,
    pub created_at: DateTime<Utc>,
}
