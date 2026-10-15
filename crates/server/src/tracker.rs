use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{error::AppResult, models::User};

pub async fn record(
    db: &SqlitePool,
    user: &User,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    detail: Option<Value>,
) {
    let id = Uuid::new_v4().to_string();
    let detail_str = detail.map(|d| d.to_string());
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO activity_log
            (id, user_id, username, action, resource_type, resource_id, detail, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&user.username)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(detail_str)
    .bind(Utc::now())
    .execute(db)
    .await
    {
        tracing::warn!("failed to record activity {action}: {e}");
    }
}

pub async fn list(
    db: &SqlitePool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<crate::models::ActivityEntry>> {
    let rows = sqlx::query_as::<_, crate::models::ActivityEntry>(
        r#"
        SELECT id, user_id, username, action, resource_type, resource_id, detail, created_at
        FROM activity_log
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    Ok(rows)
}
