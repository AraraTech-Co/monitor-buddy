use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[allow(dead_code)]
pub struct Host {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub agent_token_hash: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateHostRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateHostResponse {
    pub id: String,
    pub name: String,
    pub agent_token: String,
    pub docker_run: String,
}

#[derive(Debug, Serialize)]
pub struct HostResponse {
    pub id: String,
    pub name: String,
    pub online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub container_count: usize,
    pub unhealthy_count: usize,
    pub created_at: DateTime<Utc>,
}
