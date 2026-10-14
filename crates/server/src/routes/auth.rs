use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    auth::{hash_password, verify_password},
    error::{AppError, AppResult},
    models::{LoginRequest, RegisterRequest, User, UserResponse},
    state::AppState,
};

pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let session_id = jar
            .get("session")
            .map(|c| c.value().to_string())
            .ok_or(AppError::Unauthorized)?;

        let row = sqlx::query_as::<_, (String, String, String, chrono::DateTime<Utc>, chrono::DateTime<Utc>)>(
            r#"
            SELECT u.id, u.username, u.password_hash, u.created_at, s.expires_at
            FROM sessions s
            JOIN users u ON u.id = s.user_id
            WHERE s.id = ?
            "#,
        )
        .bind(&session_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

        let (id, username, password_hash, created_at, expires_at) = row;
        if expires_at < Utc::now() {
            let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
                .bind(&session_id)
                .execute(&state.db)
                .await;
            return Err(AppError::Unauthorized);
        }

        Ok(AuthUser(User {
            id,
            username,
            password_hash,
            created_at,
        }))
    }
}

fn session_cookie(id: &str, secure: bool, ttl_hours: i64) -> Cookie<'static> {
    let mut cookie = Cookie::new("session", id.to_string());
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(cookie::time::Duration::hours(ttl_hours));
    if secure {
        cookie.set_secure(true);
    }
    cookie
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(CookieJar, Json<UserResponse>)> {
    let username = body.username.trim().to_string();
    if username.len() < 3 || body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "username >= 3 chars and password >= 8 chars required".into(),
        ));
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    if count > 0 {
        return Err(AppError::Conflict(
            "registration closed: a user already exists".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&body.password)?;
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&username)
    .bind(&password_hash)
    .bind(now)
    .execute(&state.db)
    .await?;

    let (jar, user) = create_session(&state, jar, &id, &username).await?;
    Ok((jar, Json(user)))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<UserResponse>)> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, created_at FROM users WHERE username = ?",
    )
    .bind(body.username.trim())
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !verify_password(&body.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let (jar, resp) = create_session(&state, jar, &user.id, &user.username).await?;
    Ok((jar, Json(resp)))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<serde_json::Value>)> {
    if let Some(cookie) = jar.get("session") {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(cookie.value())
            .execute(&state.db)
            .await;
    }
    let jar = jar.remove(Cookie::from("session"));
    Ok((jar, Json(serde_json::json!({ "ok": true }))))
}

pub async fn me(AuthUser(user): AuthUser) -> Json<UserResponse> {
    Json(UserResponse {
        id: user.id,
        username: user.username,
    })
}

async fn create_session(
    state: &AppState,
    jar: CookieJar,
    user_id: &str,
    username: &str,
) -> AppResult<(CookieJar, UserResponse)> {
    let session_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires = now + Duration::hours(state.config.session_ttl_hours);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(user_id)
    .bind(expires)
    .bind(now)
    .execute(&state.db)
    .await?;

    let jar = jar.add(session_cookie(
        &session_id,
        state.config.cookie_secure,
        state.config.session_ttl_hours,
    ));

    Ok((
        jar,
        UserResponse {
            id: user_id.to_string(),
            username: username.to_string(),
        },
    ))
}
