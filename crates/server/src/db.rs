use std::path::Path;

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use crate::error::{AppError, AppResult};

pub async fn connect(database_url: &str) -> AppResult<SqlitePool> {
    if let Some(path) = database_url.strip_prefix("sqlite:") {
        let path = path.trim_start_matches("//");
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| AppError::internal(e))?;
            }
        }
    }

    let url = if database_url.contains('?') {
        format!("{database_url}&mode=rwc")
    } else if database_url.starts_with("sqlite:") {
        format!("{database_url}?mode=rwc")
    } else {
        database_url.to_string()
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await.map_err(|e| AppError::internal(e))?;
    Ok(pool)
}
