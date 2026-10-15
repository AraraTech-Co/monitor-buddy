use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::hash_password,
    error::{AppError, AppResult},
    models::{CreateUserRequest, User, UserResponse},
    routes::auth::MasterUser,
    state::AppState,
    tracker,
};

pub async fn list_users(
    State(state): State<AppState>,
    _master: MasterUser,
) -> AppResult<Json<Vec<UserResponse>>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, role, created_at FROM users ORDER BY created_at",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| UserResponse {
                id: u.id,
                username: u.username,
                role: u.role,
            })
            .collect(),
    ))
}

pub async fn create_user(
    State(state): State<AppState>,
    MasterUser(master): MasterUser,
    Json(body): Json<CreateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    let username = body.username.trim().to_string();
    if username.len() < 3 || body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "username >= 3 chars and password >= 8 chars required".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&body.password)?;
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?, ?, ?, 'member', ?)",
    )
    .bind(&id)
    .bind(&username)
    .bind(&password_hash)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db) = &e {
            if db.message().contains("UNIQUE") {
                return AppError::Conflict("username already exists".into());
            }
        }
        AppError::from(e)
    })?;

    tracker::record(
        &state.db,
        &master,
        "user.create",
        Some("user"),
        Some(&id),
        Some(serde_json::json!({ "username": username })),
    )
    .await;

    Ok(Json(UserResponse {
        id,
        username,
        role: "member".into(),
    }))
}

pub async fn delete_user(
    State(state): State<AppState>,
    MasterUser(master): MasterUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    if id == master.id {
        return Err(AppError::BadRequest("cannot delete yourself".into()));
    }

    let target = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if target.role == "master" {
        let masters: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'master'")
                .fetch_one(&state.db)
                .await?;
        if masters <= 1 {
            return Err(AppError::BadRequest("cannot delete the last master".into()));
        }
    }

    let username = target.username.clone();
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    tracker::record(
        &state.db,
        &master,
        "user.delete",
        Some("user"),
        Some(&id),
        Some(serde_json::json!({ "username": username })),
    )
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}
