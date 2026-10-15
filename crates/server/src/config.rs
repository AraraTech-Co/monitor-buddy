use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub frontend_origin: String,
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
    pub static_dir: String,
    pub public_ws_url: String,
    pub agent_image: String,
}

impl Config {
    pub fn from_env() -> Self {
        let agent_image = env::var("AGENT_IMAGE").unwrap_or_else(|_| {
            match (
                env::var("DOCKERHUB_USERNAME").ok(),
                env::var("DOCKERHUB_AGENT_REPOSITORY").ok(),
            ) {
                (Some(user), Some(repo)) if !user.is_empty() && !repo.is_empty() => {
                    format!("{user}/{repo}:latest")
                }
                _ => "monitor-buddy-agent:latest".into(),
            }
        });

        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/monitor-buddy.db".into()),
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            cookie_secure: env::var("COOKIE_SECURE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            session_ttl_hours: env::var("SESSION_TTL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(72),
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "frontend/dist".into()),
            public_ws_url: env::var("PUBLIC_WS_URL")
                .unwrap_or_else(|_| "ws://localhost:8080/agent/ws".into()),
            agent_image,
        }
    }
}
