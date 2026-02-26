pub mod jwt;
pub mod oauth;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use cookie::{Cookie, SameSite};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{errors::AppError, AppState};

// In-memory store for PKCE verifiers + CSRF tokens (keyed by CSRF state)
lazy_static::lazy_static! {
    static ref PENDING_AUTH: Mutex<HashMap<String, (oauth2::PkceCodeVerifier, std::time::Instant)>> =
        Mutex::new(HashMap::new());
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", axum::routing::post(logout))
        .route("/api/me", get(me))
}

async fn me(
    crate::extractors::AuthUser(user): crate::extractors::AuthUser,
) -> Json<crate::models::User> {
    Json(user)
}

#[derive(serde::Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

async fn login(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let client = oauth::build_oauth_client(&state.config);
    let (auth_url, csrf_token, pkce_verifier) = oauth::generate_auth_url(&client);

    {
        let mut map = PENDING_AUTH.lock().unwrap();
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(600);
        map.retain(|_, (_, inserted)| *inserted > cutoff);
        map.insert(csrf_token.secret().clone(), (pkce_verifier, std::time::Instant::now()));
    }

    Ok(Redirect::temporary(&auth_url))
}

async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackQuery>,
) -> Result<Response, AppError> {
    // Retrieve and remove PKCE verifier
    let pkce_verifier = {
        let mut map = PENDING_AUTH.lock().unwrap();
        map.remove(&params.state).map(|(v, _)| v)
    }
    .ok_or_else(|| AppError::BadRequest("Invalid or expired auth state".into()))?;

    // Exchange code for access token
    let client = oauth::build_oauth_client(&state.config);
    let access_token = oauth::exchange_code(&client, params.code, pkce_verifier)
        .await
        .map_err(AppError::Internal)?;

    // Fetch user info from Authentik
    let user_info = oauth::fetch_user_info(&state.config.authentik_base_url, &access_token)
        .await
        .map_err(AppError::Internal)?;

    // Upsert user in database
    let user_id = ulid::Ulid::new().to_string();
    let display_name = user_info.name.unwrap_or_else(|| {
        user_info
            .preferred_username
            .unwrap_or_else(|| user_info.email.clone())
    });

    sqlx::query(
        "INSERT INTO users (id, email, display_name, avatar_url)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(email) DO UPDATE SET
            display_name = excluded.display_name,
            avatar_url = excluded.avatar_url,
            updated_at = datetime('now')",
    )
    .bind(&user_id)
    .bind(&user_info.email)
    .bind(&display_name)
    .bind(&user_info.picture)
    .execute(&state.db.write)
    .await?;

    // Get the actual user ID (might be existing user)
    let actual_user: crate::models::User =
        sqlx::query_as("SELECT * FROM users WHERE email = ?1")
            .bind(&user_info.email)
            .fetch_one(&state.db.read)
            .await?;

    // Create session
    let session_id = ulid::Ulid::new().to_string();
    let refresh_token: String = {
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    };
    let refresh_hash = hex::encode(Sha256::digest(refresh_token.as_bytes()));
    let expires_at = chrono::Utc::now() + chrono::TimeDelta::days(30);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&session_id)
    .bind(&actual_user.id)
    .bind(&refresh_hash)
    .bind(expires_at.to_rfc3339())
    .execute(&state.db.write)
    .await?;

    // Auto-claim any pending invites for this email
    claim_pending_invites(&state, &user_info.email, &actual_user.id).await?;

    // Create JWT
    let jwt = jwt::create_access_token(
        &actual_user.id,
        &session_id,
        &actual_user.email,
        &state.config.jwt_secret,
    )
    .map_err(|e| AppError::Internal(format!("JWT creation failed: {e}")))?;

    // Set cookies and redirect to app
    let access_cookie = Cookie::build(("rice_access", jwt))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::minutes(15));

    let refresh_cookie = Cookie::build(("rice_refresh", refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/auth")
        .max_age(cookie::time::Duration::days(30));

    let mut response = Redirect::temporary("/").into_response();
    let headers = response.headers_mut();
    headers.append(
        axum::http::header::SET_COOKIE,
        access_cookie.to_string().parse().unwrap(),
    );
    headers.append(
        axum::http::header::SET_COOKIE,
        refresh_cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, AppError> {
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie_part in cookie_str.split(';') {
                let cookie_part = cookie_part.trim();
                if let Ok(c) = Cookie::parse(cookie_part) {
                    if c.name() == "rice_access" {
                        if let Ok(claims) =
                            jwt::decode_ignoring_expiry(c.value(), &state.config.jwt_secret)
                        {
                            let _ = sqlx::query("DELETE FROM sessions WHERE id = ?1")
                                .bind(&claims.jti)
                                .execute(&state.db.write)
                                .await;
                        }
                    }
                }
            }
        }
    }

    let clear_access = Cookie::build(("rice_access", ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::ZERO);

    let clear_refresh = Cookie::build(("rice_refresh", ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/auth")
        .max_age(cookie::time::Duration::ZERO);

    let mut response = StatusCode::OK.into_response();
    let headers = response.headers_mut();
    headers.append(
        axum::http::header::SET_COOKIE,
        clear_access.to_string().parse().unwrap(),
    );
    headers.append(
        axum::http::header::SET_COOKIE,
        clear_refresh.to_string().parse().unwrap(),
    );

    Ok(response)
}

async fn claim_pending_invites(
    state: &AppState,
    email: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let invites: Vec<crate::models::Invite> = sqlx::query_as(
        "SELECT * FROM invites WHERE email = ?1 AND claimed_by IS NULL AND expires_at > datetime('now')",
    )
    .bind(email)
    .fetch_all(&state.db.read)
    .await?;

    for invite in invites {
        let mut tx = state.db.write.begin().await?;
        sqlx::query(
            "INSERT OR IGNORE INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, ?3)",
        )
        .bind(&invite.trip_id)
        .bind(user_id)
        .bind(&invite.role)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE invites SET claimed_by = ?1 WHERE id = ?2")
            .bind(user_id)
            .bind(&invite.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }

    Ok(())
}
