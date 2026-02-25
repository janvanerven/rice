# Rice (旅) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a self-hosted trip management webapp with Japanese cyberpunk aesthetic, Authentik OAuth2 auth, and collaborative trip planning.

**Architecture:** Rust (Axum) backend serves a React (Vite) SPA from a single binary. SQLite for storage. OAuth2 PKCE with Authentik for identity. Docker for deployment.

**Tech Stack:** Rust, Axum, sqlx, SQLite, React, Vite, React Router, CSS Modules, Docker

**Design Doc:** `docs/plans/2026-02-25-rice-design.md`

---

## Project Structure

```
rice/
├── backend/
│   ├── Cargo.toml
│   ├── .sqlx/                    # Offline query cache (committed)
│   ├── migrations/
│   │   └── 0001_initial.sql
│   ├── src/
│   │   ├── main.rs               # Entry point, startup validation, server
│   │   ├── config.rs             # Env var loading + validation
│   │   ├── db.rs                 # Pool setup (1 write, N read)
│   │   ├── models.rs             # Domain types (User, Trip, etc.)
│   │   ├── auth/
│   │   │   ├── mod.rs            # Auth routes
│   │   │   ├── oauth.rs          # Authentik PKCE client
│   │   │   └── jwt.rs            # JWT sign/verify, cookie management
│   │   ├── api/
│   │   │   ├── mod.rs            # API router
│   │   │   ├── trips.rs          # Trip CRUD handlers
│   │   │   ├── members.rs        # Member management
│   │   │   └── invites.rs        # Invite creation + claiming
│   │   ├── extractors.rs         # AuthUser, TripAccess extractors
│   │   ├── email.rs              # SMTP client for invites
│   │   ├── errors.rs             # AppError type + IntoResponse
│   │   └── middleware.rs         # Security headers, rate limiting
│   └── tests/
│       ├── common/mod.rs         # Test helpers (test DB, mock auth)
│       ├── api_trips_test.rs
│       ├── api_members_test.rs
│       └── auth_test.rs
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── globals.css           # Design tokens, reset, animations
│       ├── types/index.ts        # TypeScript interfaces
│       ├── lib/
│       │   ├── api.ts            # Fetch wrapper
│       │   └── auth.ts           # Auth helpers
│       ├── hooks/
│       │   ├── useAuth.ts
│       │   ├── useTrips.ts
│       │   └── useMediaQuery.ts
│       ├── components/
│       │   ├── ui/
│       │   │   ├── Button/
│       │   │   ├── Input/
│       │   │   ├── Card/
│       │   │   ├── Badge/
│       │   │   ├── Modal/
│       │   │   └── GlowDivider/
│       │   ├── layout/
│       │   │   ├── AppShell.tsx
│       │   │   ├── AppShell.module.css
│       │   │   ├── BottomNav.tsx
│       │   │   ├── SideNav.tsx
│       │   │   └── PageHeader.tsx
│       │   ├── trips/
│       │   │   ├── TripCard.tsx
│       │   │   ├── TripCard.module.css
│       │   │   ├── TripGrid.tsx
│       │   │   ├── TripForm.tsx
│       │   │   ├── TripDetail.tsx
│       │   │   └── CollaboratorList.tsx
│       │   └── auth/
│       │       └── LoginScreen.tsx
│       └── pages/
│           ├── LoginPage.tsx
│           ├── DashboardPage.tsx
│           ├── TripNewPage.tsx
│           ├── TripDetailPage.tsx
│           └── InviteClaimPage.tsx
├── Dockerfile
├── docker-compose.yml
└── docs/plans/
```

---

## Task 1: Rust Project Scaffolding

**Files:**
- Create: `backend/Cargo.toml`
- Create: `backend/src/main.rs`
- Create: `backend/src/config.rs`
- Create: `backend/src/errors.rs`

**Step 1: Initialize Rust project**

```bash
cd /home/jan/rice
cargo init backend
```

**Step 2: Set up Cargo.toml with all dependencies**

Replace `backend/Cargo.toml` with:

```toml
[package]
name = "rice"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework
axum = { version = "0.8", features = ["macros", "multipart"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "fs", "trace", "set-header"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }

# Auth
jsonwebtoken = "9"
oauth2 = "5.0.0-rc.1"
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# IDs + crypto
ulid = { version = "1", features = ["serde"] }
sha2 = "0.10"
hex = "0.4"
rand = "0.8"
base64 = "0.22"

# Email
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }

# Config + logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Static file embedding
rust-embed = "8"
mime_guess = "2"

# Cookie handling
cookie = "0.18"

[dev-dependencies]
axum-test = "16"
tempfile = "3"
```

**Step 3: Write config.rs — env var loading with validation**

```rust
// backend/src/config.rs
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub authentik_client_id: String,
    pub authentik_client_secret: String,
    pub authentik_base_url: String,
    pub app_base_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret = require_env("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 bytes".into());
        }

        Ok(Config {
            database_url: require_env("DATABASE_URL")?,
            jwt_secret,
            authentik_client_id: require_env("AUTHENTIK_CLIENT_ID")?,
            authentik_client_secret: require_env("AUTHENTIK_CLIENT_SECRET")?,
            authentik_base_url: require_env("AUTHENTIK_BASE_URL")?,
            app_base_url: require_env("APP_BASE_URL")?,
            smtp_host: require_env("SMTP_HOST")?,
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".into())
                .parse()
                .map_err(|_| "SMTP_PORT must be a number")?,
            smtp_username: require_env("SMTP_USERNAME")?,
            smtp_password: require_env("SMTP_PASSWORD")?,
            smtp_from: require_env("SMTP_FROM")?,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .map_err(|_| "PORT must be a number")?,
        })
    }
}

fn require_env(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("Missing required env var: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_env_missing() {
        let result = require_env("DEFINITELY_NOT_SET_12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DEFINITELY_NOT_SET_12345"));
    }
}
```

**Step 4: Write errors.rs — unified error type**

```rust
// backend/src/errors.rs
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into())
            }
        };

        (status, axum::Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
```

**Step 5: Write minimal main.rs — startup with validation**

```rust
// backend/src/main.rs
mod config;
mod errors;

use config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rice=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env().unwrap_or_else(|err| {
        tracing::error!("Configuration error: {err}");
        std::process::exit(1);
    });

    tracing::info!("Rice starting on {}:{}", config.host, config.port);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .expect("Failed to bind address");

    tracing::info!("Listening on {}", listener.local_addr().unwrap());

    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }));

    axum::serve(listener, app).await.expect("Server failed");
}
```

**Step 6: Verify it compiles**

```bash
cd /home/jan/rice/backend && cargo check
```

**Step 7: Commit**

```bash
git add backend/ && git commit -m "feat: scaffold Rust backend with config, errors, health check"
```

---

## Task 2: Database Layer

**Files:**
- Create: `backend/src/db.rs`
- Create: `backend/src/models.rs`
- Create: `backend/migrations/0001_initial.sql`
- Modify: `backend/src/main.rs`

**Step 1: Write the initial migration**

```sql
-- backend/migrations/0001_initial.sql

-- Users (synced from Authentik on login)
CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,           -- ULID
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Trips
CREATE TABLE trips (
    id TEXT PRIMARY KEY NOT NULL,           -- ULID
    name TEXT NOT NULL,
    destination TEXT NOT NULL DEFAULT '',
    start_date TEXT,                         -- ISO date, nullable
    end_date TEXT,                           -- ISO date, nullable
    cover_image_path TEXT,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Trip members (who can access a trip)
CREATE TABLE trip_members (
    trip_id TEXT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (trip_id, user_id)
);

-- Invites (pending trip invitations)
CREATE TABLE invites (
    id TEXT PRIMARY KEY NOT NULL,           -- ULID
    trip_id TEXT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('editor', 'viewer')),
    expires_at TEXT NOT NULL,
    claimed_by TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions (for JWT revocation)
CREATE TABLE sessions (
    id TEXT PRIMARY KEY NOT NULL,           -- JTI
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes
CREATE INDEX idx_trip_members_user ON trip_members(user_id);
CREATE INDEX idx_invites_email ON invites(email);
CREATE INDEX idx_invites_token ON invites(token_hash);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
```

**Step 2: Write db.rs — pool setup with WAL mode**

```rust
// backend/src/db.rs
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;

pub struct DbPools {
    pub write: SqlitePool,
    pub read: SqlitePool,
}

impl DbPools {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let base_opts = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .pragma("synchronous", "NORMAL")
            .pragma("foreign_keys", "ON")
            .create_if_missing(true);

        let write = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(base_opts.clone())
            .await?;

        let read = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(base_opts.read_only(true))
            .await?;

        // Run migrations on the write pool
        sqlx::migrate!("./migrations").run(&write).await?;

        tracing::info!("Database connected and migrated");

        Ok(DbPools { write, read })
    }
}
```

**Step 3: Write models.rs — domain types**

```rust
// backend/src/models.rs
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trip {
    pub id: String,
    pub name: String,
    pub destination: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub cover_image_path: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TripMember {
    pub trip_id: String,
    pub user_id: String,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripWithRole {
    #[serde(flatten)]
    pub trip: Trip,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invite {
    pub id: String,
    pub trip_id: String,
    pub email: String,
    pub token_hash: String,
    pub role: String,
    pub expires_at: String,
    pub claimed_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub refresh_token_hash: String,
    pub expires_at: String,
    pub created_at: String,
}

// API request/response types

#[derive(Debug, Deserialize)]
pub struct CreateTripRequest {
    pub name: String,
    pub destination: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTripRequest {
    pub name: Option<String>,
    pub destination: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: String,
}
```

**Step 4: Update main.rs to connect DB**

Update `backend/src/main.rs` to add `mod db; mod models;` and connect the database on startup, passing `DbPools` as app state.

```rust
// backend/src/main.rs
mod config;
mod db;
mod errors;
mod models;

use config::Config;
use db::DbPools;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbPools>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rice=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env().unwrap_or_else(|err| {
        tracing::error!("Configuration error: {err}");
        std::process::exit(1);
    });

    let db = DbPools::connect(&config.database_url).await.unwrap_or_else(|err| {
        tracing::error!("Database connection failed: {err}");
        std::process::exit(1);
    });

    let state = AppState {
        config: config.clone(),
        db: Arc::new(db),
    };

    tracing::info!("Rice starting on {}:{}", config.host, config.port);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .expect("Failed to bind address");

    let app = axum::Router::new()
        .route("/health", axum::routing::get(health))
        .with_state(state);

    axum::serve(listener, app).await.expect("Server failed");
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<&'static str, errors::AppError> {
    sqlx::query("SELECT 1")
        .execute(&state.db.read)
        .await?;
    Ok("ok")
}
```

**Step 5: Create test database, verify migration runs**

```bash
cd /home/jan/rice/backend
DATABASE_URL="sqlite:///tmp/rice_test.db" cargo sqlx migrate run
```

**Step 6: Prepare sqlx offline cache**

```bash
cd /home/jan/rice/backend
DATABASE_URL="sqlite:///tmp/rice_test.db" cargo sqlx prepare
```

**Step 7: Verify compilation**

```bash
cd /home/jan/rice/backend && cargo check
```

**Step 8: Commit**

```bash
git add backend/ && git commit -m "feat: add database layer with SQLite, migrations, and domain models"
```

---

## Task 3: Auth — JWT Module

**Files:**
- Create: `backend/src/auth/mod.rs`
- Create: `backend/src/auth/jwt.rs`
- Create: `backend/src/auth/oauth.rs`
- Create: `backend/src/extractors.rs`
- Modify: `backend/src/main.rs`

**Step 1: Write JWT signing/verification**

```rust
// backend/src/auth/jwt.rs
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // user_id
    pub jti: String,       // session_id for revocation
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn create_access_token(
    user_id: &str,
    session_id: &str,
    email: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        jti: session_id.to_string(),
        email: email.to_string(),
        exp: (now + Duration::minutes(15)).timestamp(),
        iat: now.timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let secret = "a]very]secret]key]that]is]at]least]32]bytes";
        let token = create_access_token("user123", "session456", "test@example.com", secret)
            .unwrap();
        let claims = verify_token(&token, secret).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.jti, "session456");
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_verify_with_wrong_secret() {
        let token = create_access_token("user123", "session456", "test@example.com", "a]very]secret]key]that]is]at]least]32]bytes")
            .unwrap();
        let result = verify_token(&token, "wrong_secret_that_is_also_32_bytes_long!");
        assert!(result.is_err());
    }
}
```

**Step 2: Write OAuth2 PKCE client**

```rust
// backend/src/auth/oauth.rs
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
    PkceCodeChallenge, PkceCodeVerifier, CsrfToken, Scope, AuthorizationCode,
    TokenResponse, reqwest::async_http_client,
};
use serde::Deserialize;

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct AuthentikUserInfo {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub picture: Option<String>,
}

pub fn build_oauth_client(config: &Config) -> BasicClient {
    let auth_url = format!(
        "{}/application/o/authorize/",
        config.authentik_base_url.trim_end_matches('/')
    );
    let token_url = format!(
        "{}/application/o/token/",
        config.authentik_base_url.trim_end_matches('/')
    );
    let redirect_url = format!(
        "{}/auth/callback",
        config.app_base_url.trim_end_matches('/')
    );

    BasicClient::new(
        ClientId::new(config.authentik_client_id.clone()),
        Some(ClientSecret::new(config.authentik_client_secret.clone())),
        AuthUrl::new(auth_url).expect("Invalid auth URL"),
        Some(TokenUrl::new(token_url).expect("Invalid token URL")),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"))
}

pub fn generate_auth_url(client: &BasicClient) -> (String, CsrfToken, PkceCodeVerifier) {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("profile".into()))
        .add_scope(Scope::new("email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url.to_string(), csrf_token, pkce_verifier)
}

pub async fn exchange_code(
    client: &BasicClient,
    code: String,
    pkce_verifier: PkceCodeVerifier,
) -> Result<String, String> {
    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Token exchange failed: {e}"))?;

    Ok(token_result.access_token().secret().clone())
}

pub async fn fetch_user_info(
    authentik_base_url: &str,
    access_token: &str,
) -> Result<AuthentikUserInfo, String> {
    let url = format!(
        "{}/application/o/userinfo/",
        authentik_base_url.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Userinfo request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Userinfo returned status: {}", resp.status()));
    }

    resp.json::<AuthentikUserInfo>()
        .await
        .map_err(|e| format!("Failed to parse userinfo: {e}"))
}
```

**Step 3: Write auth routes (login, callback, logout)**

```rust
// backend/src/auth/mod.rs
pub mod jwt;
pub mod oauth;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use cookie::{Cookie, SameSite};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{errors::AppError, AppState};

// In-memory store for PKCE verifiers + CSRF tokens (keyed by CSRF state)
// In production, this should be in the DB or a short-lived cache,
// but for a self-hosted single-instance app this is fine.
lazy_static::lazy_static! {
    static ref PENDING_AUTH: Mutex<HashMap<String, oauth2::PkceCodeVerifier>> =
        Mutex::new(HashMap::new());
}

// Note: We need to add lazy_static to Cargo.toml

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", axum::routing::post(logout))
}

#[derive(serde::Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

async fn login(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let client = oauth::build_oauth_client(&state.config);
    let (auth_url, csrf_token, pkce_verifier) = oauth::generate_auth_url(&client);

    // Store PKCE verifier keyed by CSRF state
    PENDING_AUTH
        .lock()
        .unwrap()
        .insert(csrf_token.secret().clone(), pkce_verifier);

    Ok(Redirect::temporary(&auth_url))
}

async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackQuery>,
) -> Result<Response, AppError> {
    // Retrieve and remove PKCE verifier
    let pkce_verifier = PENDING_AUTH
        .lock()
        .unwrap()
        .remove(&params.state)
        .ok_or_else(|| AppError::BadRequest("Invalid or expired auth state".into()))?;

    // Exchange code for access token
    let client = oauth::build_oauth_client(&state.config);
    let access_token = oauth::exchange_code(&client, params.code, pkce_verifier)
        .await
        .map_err(|e| AppError::Internal(e))?;

    // Fetch user info from Authentik
    let user_info = oauth::fetch_user_info(&state.config.authentik_base_url, &access_token)
        .await
        .map_err(|e| AppError::Internal(e))?;

    // Upsert user in database
    let user_id = ulid::Ulid::new().to_string();
    let display_name = user_info
        .name
        .unwrap_or_else(|| user_info.preferred_username.unwrap_or_else(|| user_info.email.clone()));

    sqlx::query(
        "INSERT INTO users (id, email, display_name, avatar_url)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(email) DO UPDATE SET
            display_name = excluded.display_name,
            avatar_url = excluded.avatar_url,
            updated_at = datetime('now')"
    )
    .bind(&user_id)
    .bind(&user_info.email)
    .bind(&display_name)
    .bind(&user_info.picture)
    .execute(&state.db.write)
    .await?;

    // Get the actual user ID (might be existing user)
    let actual_user: crate::models::User = sqlx::query_as(
        "SELECT * FROM users WHERE email = ?1"
    )
    .bind(&user_info.email)
    .fetch_one(&state.db.read)
    .await?;

    // Create session
    let session_id = ulid::Ulid::new().to_string();
    let refresh_token: String = {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    };
    let refresh_hash = hex::encode(Sha256::digest(refresh_token.as_bytes()));
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at) VALUES (?1, ?2, ?3, ?4)"
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

async fn logout(State(state): State<AppState>, headers: axum::http::HeaderMap) -> Result<Response, AppError> {
    // Extract session ID from JWT cookie
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie_part in cookie_str.split(';') {
                let cookie_part = cookie_part.trim();
                if let Ok(c) = Cookie::parse(cookie_part) {
                    if c.name() == "rice_access" {
                        if let Ok(claims) = jwt::verify_token(c.value(), &state.config.jwt_secret) {
                            // Delete session
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

    // Clear cookies
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
        "SELECT * FROM invites WHERE email = ?1 AND claimed_by IS NULL AND expires_at > datetime('now')"
    )
    .bind(email)
    .fetch_all(&state.db.read)
    .await?;

    for invite in invites {
        // Add as trip member
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, ?3)"
        )
        .bind(&invite.trip_id)
        .bind(user_id)
        .bind(&invite.role)
        .execute(&state.db.write)
        .await;

        // Mark invite as claimed
        let _ = sqlx::query(
            "UPDATE invites SET claimed_by = ?1 WHERE id = ?2"
        )
        .bind(user_id)
        .bind(&invite.id)
        .execute(&state.db.write)
        .await;
    }

    Ok(())
}
```

**Step 4: Write AuthUser extractor**

```rust
// backend/src/extractors.rs
use axum::{
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
};
use cookie::Cookie;

use crate::{auth::jwt, errors::AppError, models::User, AppState};

/// Extracts the authenticated user from the JWT cookie.
/// Returns 401 if no valid session.
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookie_header = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

        let mut access_token = None;
        for cookie_part in cookie_header.split(';') {
            if let Ok(c) = Cookie::parse(cookie_part.trim()) {
                if c.name() == "rice_access" {
                    access_token = Some(c.value().to_string());
                }
            }
        }

        let token = access_token
            .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

        let claims = jwt::verify_token(&token, &state.config.jwt_secret)
            .map_err(|_| AppError::Unauthorized("Invalid or expired token".into()))?;

        // Verify session still exists (revocation check)
        let session_exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sessions WHERE id = ?1 AND expires_at > datetime('now')"
        )
        .bind(&claims.jti)
        .fetch_one(&state.db.read)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if !session_exists {
            return Err(AppError::Unauthorized("Session revoked or expired".into()));
        }

        // Fetch user
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
            .bind(&claims.sub)
            .fetch_optional(&state.db.read)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("User not found".into()))?;

        Ok(AuthUser(user))
    }
}

/// Extracts authenticated user + verifies trip membership.
/// Must be used with a path parameter `:trip_id`.
pub struct TripAccess {
    pub user: User,
    pub trip_id: String,
    pub role: String,
}

impl TripAccess {
    pub fn require_editor(&self) -> Result<(), AppError> {
        match self.role.as_str() {
            "owner" | "editor" => Ok(()),
            _ => Err(AppError::Forbidden("Editor access required".into())),
        }
    }

    pub fn require_owner(&self) -> Result<(), AppError> {
        if self.role == "owner" {
            Ok(())
        } else {
            Err(AppError::Forbidden("Owner access required".into()))
        }
    }
}

impl FromRequestParts<AppState> for TripAccess {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;

        // Extract trip_id from path
        let Path(params): Path<std::collections::HashMap<String, String>> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::BadRequest("Missing trip_id in path".into()))?;

        let trip_id = params
            .get("trip_id")
            .ok_or_else(|| AppError::BadRequest("Missing trip_id parameter".into()))?
            .clone();

        // Check membership
        let member: Option<crate::models::TripMember> = sqlx::query_as(
            "SELECT * FROM trip_members WHERE trip_id = ?1 AND user_id = ?2"
        )
        .bind(&trip_id)
        .bind(&user.id)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let member = member.ok_or_else(|| AppError::NotFound("Trip not found".into()))?;

        Ok(TripAccess {
            user,
            trip_id,
            role: member.role,
        })
    }
}
```

**Step 5: Add `lazy_static` to Cargo.toml**

Add to `[dependencies]`: `lazy_static = "1"`

**Step 6: Wire auth routes into main.rs**

Update `main.rs` to add `mod auth; mod extractors;` and merge the auth router:

```rust
// In main.rs, update the app builder:
let app = axum::Router::new()
    .route("/health", axum::routing::get(health))
    .merge(auth::router())
    .with_state(state);
```

**Step 7: Run tests**

```bash
cd /home/jan/rice/backend && cargo test
```

**Step 8: Commit**

```bash
git add backend/ && git commit -m "feat: add OAuth2 PKCE auth, JWT sessions, AuthUser and TripAccess extractors"
```

---

## Task 4: Trip CRUD API

**Files:**
- Create: `backend/src/api/mod.rs`
- Create: `backend/src/api/trips.rs`
- Modify: `backend/src/main.rs`

**Step 1: Write trip CRUD handlers**

```rust
// backend/src/api/trips.rs
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::{
    errors::AppError,
    extractors::{AuthUser, TripAccess},
    models::{CreateTripRequest, Trip, TripWithRole, UpdateTripRequest},
    AppState,
};

pub async fn list_trips(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<TripWithRole>>, AppError> {
    let trips: Vec<TripWithRole> = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<String>, String, String, String, String)>(
        "SELECT t.id, t.name, t.destination, t.start_date, t.end_date, t.cover_image_path, t.created_by, t.created_at, t.updated_at, tm.role
         FROM trips t
         JOIN trip_members tm ON tm.trip_id = t.id
         WHERE tm.user_id = ?1
         ORDER BY t.created_at DESC"
    )
    .bind(&user.id)
    .fetch_all(&state.db.read)
    .await?
    .into_iter()
    .map(|row| TripWithRole {
        trip: Trip {
            id: row.0,
            name: row.1,
            destination: row.2,
            start_date: row.3,
            end_date: row.4,
            cover_image_path: row.5,
            created_by: row.6,
            created_at: row.7,
            updated_at: row.8,
        },
        role: row.9,
    })
    .collect();

    Ok(Json(trips))
}

pub async fn create_trip(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateTripRequest>,
) -> Result<(StatusCode, Json<Trip>), AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("Trip name is required".into()));
    }

    let id = ulid::Ulid::new().to_string();

    sqlx::query(
        "INSERT INTO trips (id, name, destination, start_date, end_date, created_by) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(&id)
    .bind(req.name.trim())
    .bind(req.destination.as_deref().unwrap_or(""))
    .bind(&req.start_date)
    .bind(&req.end_date)
    .bind(&user.id)
    .execute(&state.db.write)
    .await?;

    // Add creator as owner
    sqlx::query(
        "INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')"
    )
    .bind(&id)
    .bind(&user.id)
    .execute(&state.db.write)
    .await?;

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&id)
        .fetch_one(&state.db.read)
        .await?;

    Ok((StatusCode::CREATED, Json(trip)))
}

pub async fn get_trip(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<TripWithRole>, AppError> {
    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    Ok(Json(TripWithRole {
        trip,
        role: access.role,
    }))
}

pub async fn update_trip(
    State(state): State<AppState>,
    access: TripAccess,
    Json(req): Json<UpdateTripRequest>,
) -> Result<Json<Trip>, AppError> {
    access.require_editor()?;

    // Build dynamic update
    let mut sets = vec!["updated_at = datetime('now')"];
    let mut binds: Vec<String> = vec![];

    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest("Trip name cannot be empty".into()));
        }
        sets.push("name = ?");
        binds.push(name.trim().to_string());
    }
    if let Some(ref dest) = req.destination {
        sets.push("destination = ?");
        binds.push(dest.clone());
    }
    // For dates we need a different approach since they're optional
    // Use raw query with explicit bind positions

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    let name = req.name.unwrap_or(trip.name);
    let destination = req.destination.unwrap_or(trip.destination);
    let start_date = req.start_date.or(trip.start_date);
    let end_date = req.end_date.or(trip.end_date);

    sqlx::query(
        "UPDATE trips SET name = ?1, destination = ?2, start_date = ?3, end_date = ?4, updated_at = datetime('now') WHERE id = ?5"
    )
    .bind(name.trim())
    .bind(&destination)
    .bind(&start_date)
    .bind(&end_date)
    .bind(&access.trip_id)
    .execute(&state.db.write)
    .await?;

    let updated: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    Ok(Json(updated))
}

pub async fn delete_trip(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<StatusCode, AppError> {
    access.require_owner()?;

    sqlx::query("DELETE FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .execute(&state.db.write)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
```

**Step 2: Write API router**

```rust
// backend/src/api/mod.rs
pub mod trips;

use axum::{routing::{get, post, put, delete}, Router};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/trips", get(trips::list_trips).post(trips::create_trip))
        .route(
            "/api/trips/{trip_id}",
            get(trips::get_trip)
                .put(trips::update_trip)
                .delete(trips::delete_trip),
        )
}
```

**Step 3: Wire into main.rs**

Add `mod api;` and merge the API router:

```rust
let app = axum::Router::new()
    .route("/health", axum::routing::get(health))
    .merge(auth::router())
    .merge(api::router())
    .with_state(state);
```

**Step 4: Verify compilation**

```bash
cd /home/jan/rice/backend && cargo check
```

**Step 5: Commit**

```bash
git add backend/ && git commit -m "feat: add trip CRUD API endpoints"
```

---

## Task 5: Members & Invites API

**Files:**
- Create: `backend/src/api/members.rs`
- Create: `backend/src/api/invites.rs`
- Create: `backend/src/email.rs`
- Modify: `backend/src/api/mod.rs`

**Step 1: Write members handler**

```rust
// backend/src/api/members.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{errors::AppError, extractors::TripAccess, models::MemberResponse, AppState};

pub async fn list_members(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<Vec<MemberResponse>>, AppError> {
    let members: Vec<MemberResponse> = sqlx::query_as::<_, (String, String, String, Option<String>, String, String)>(
        "SELECT u.id, u.email, u.display_name, u.avatar_url, tm.role, tm.joined_at
         FROM trip_members tm
         JOIN users u ON u.id = tm.user_id
         WHERE tm.trip_id = ?1
         ORDER BY tm.joined_at"
    )
    .bind(&access.trip_id)
    .fetch_all(&state.db.read)
    .await?
    .into_iter()
    .map(|row| MemberResponse {
        user_id: row.0,
        email: row.1,
        display_name: row.2,
        avatar_url: row.3,
        role: row.4,
        joined_at: row.5,
    })
    .collect();

    Ok(Json(members))
}

#[derive(serde::Deserialize)]
pub struct RemoveMemberPath {
    pub trip_id: String,
    pub user_id: String,
}

pub async fn remove_member(
    State(state): State<AppState>,
    access: TripAccess,
    Path(params): Path<RemoveMemberPath>,
) -> Result<StatusCode, AppError> {
    access.require_owner()?;

    // Can't remove yourself as owner
    if params.user_id == access.user.id {
        return Err(AppError::BadRequest("Cannot remove yourself as owner".into()));
    }

    let result = sqlx::query(
        "DELETE FROM trip_members WHERE trip_id = ?1 AND user_id = ?2"
    )
    .bind(&access.trip_id)
    .bind(&params.user_id)
    .execute(&state.db.write)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}
```

**Step 2: Write email service**

```rust
// backend/src/email.rs
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::config::Config;

pub struct EmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl EmailService {
    pub fn new(config: &Config) -> Result<Self, String> {
        let creds = Credentials::new(
            config.smtp_username.clone(),
            config.smtp_password.clone(),
        );

        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            .map_err(|e| format!("SMTP setup failed: {e}"))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

        Ok(EmailService {
            transport,
            from: config.smtp_from.clone(),
        })
    }

    pub async fn send_invite(
        &self,
        to_email: &str,
        trip_name: &str,
        inviter_name: &str,
        invite_url: &str,
    ) -> Result<(), String> {
        let body = format!(
            "{inviter_name} invited you to join the trip \"{trip_name}\" on Rice.\n\n\
             Click here to join: {invite_url}\n\n\
             This invite expires in 7 days."
        );

        let email = Message::builder()
            .from(self.from.parse().map_err(|e| format!("Invalid from address: {e}"))?)
            .to(to_email.parse().map_err(|e| format!("Invalid to address: {e}"))?)
            .subject(format!("You're invited to join \"{trip_name}\" on Rice"))
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| format!("Failed to build email: {e}"))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| format!("Failed to send email: {e}"))?;

        Ok(())
    }
}
```

**Step 3: Write invites handler**

```rust
// backend/src/api/invites.rs
use axum::{extract::State, http::StatusCode, Json};
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::{errors::AppError, extractors::TripAccess, models::CreateInviteRequest, AppState};

pub async fn create_invite(
    State(state): State<AppState>,
    access: TripAccess,
    Json(req): Json<CreateInviteRequest>,
) -> Result<StatusCode, AppError> {
    access.require_owner()?;

    // Validate role
    if !["editor", "viewer"].contains(&req.role.as_str()) {
        return Err(AppError::BadRequest("Role must be 'editor' or 'viewer'".into()));
    }

    // Check if user is already a member
    let already_member: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM trip_members tm
         JOIN users u ON u.id = tm.user_id
         WHERE tm.trip_id = ?1 AND u.email = ?2"
    )
    .bind(&access.trip_id)
    .bind(&req.email)
    .fetch_one(&state.db.read)
    .await
    .unwrap_or(false);

    if already_member {
        return Err(AppError::BadRequest("User is already a member of this trip".into()));
    }

    // Generate token
    let token: String = {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    };
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    let id = ulid::Ulid::new().to_string();

    sqlx::query(
        "INSERT INTO invites (id, trip_id, email, token_hash, role, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(&id)
    .bind(&access.trip_id)
    .bind(&req.email)
    .bind(&token_hash)
    .bind(&req.role)
    .bind(expires_at.to_rfc3339())
    .execute(&state.db.write)
    .await?;

    // Send invite email
    let invite_url = format!(
        "{}/invite?token={}",
        state.config.app_base_url.trim_end_matches('/'),
        token
    );

    // Get trip name for the email
    let trip_name: String = sqlx::query_scalar("SELECT name FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    if let Some(ref email_service) = state.email {
        if let Err(e) = email_service
            .send_invite(&req.email, &trip_name, &access.user.display_name, &invite_url)
            .await
        {
            tracing::warn!("Failed to send invite email: {e}");
            // Don't fail the request — invite is created, email is best-effort
        }
    }

    Ok(StatusCode::CREATED)
}
```

**Step 4: Update AppState to include EmailService**

In `main.rs`, add `email` field to `AppState`:

```rust
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbPools>,
    pub email: Option<Arc<email::EmailService>>,
}
```

Initialize it in main:

```rust
let email_service = match email::EmailService::new(&config) {
    Ok(svc) => Some(Arc::new(svc)),
    Err(e) => {
        tracing::warn!("Email service not available: {e}");
        None
    }
};

let state = AppState {
    config: config.clone(),
    db: Arc::new(db),
    email: email_service,
};
```

**Step 5: Update API router with member/invite routes**

```rust
// backend/src/api/mod.rs
pub mod invites;
pub mod members;
pub mod trips;

use axum::{routing::{get, post, put, delete}, Router};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/trips", get(trips::list_trips).post(trips::create_trip))
        .route(
            "/api/trips/{trip_id}",
            get(trips::get_trip)
                .put(trips::update_trip)
                .delete(trips::delete_trip),
        )
        .route(
            "/api/trips/{trip_id}/members",
            get(members::list_members),
        )
        .route(
            "/api/trips/{trip_id}/members/{user_id}",
            delete(members::remove_member),
        )
        .route(
            "/api/trips/{trip_id}/invites",
            post(invites::create_invite),
        )
}
```

**Step 6: Add `/api/me` endpoint**

Add to `backend/src/auth/mod.rs`:

```rust
async fn me(AuthUser(user): AuthUser) -> Json<User> {
    Json(user)
}
```

And add to the auth router:

```rust
.route("/api/me", get(me))
```

(Will need to import `AuthUser` and `User` and `Json` at the top of auth/mod.rs)

**Step 7: Verify compilation**

```bash
cd /home/jan/rice/backend && cargo check
```

**Step 8: Commit**

```bash
git add backend/ && git commit -m "feat: add members, invites API with SMTP email service"
```

---

## Task 6: Security Middleware

**Files:**
- Create: `backend/src/middleware.rs`
- Modify: `backend/src/main.rs`
- Modify: `backend/Cargo.toml`

**Step 1: Write security headers middleware**

```rust
// backend/src/middleware.rs
use axum::http::{header, HeaderValue};
use tower_http::set_header::SetResponseHeaderLayer;

pub fn security_headers() -> tower::ServiceBuilder<
    tower::layer::util::Stack<
        SetResponseHeaderLayer<HeaderValue>,
        tower::layer::util::Stack<
            SetResponseHeaderLayer<HeaderValue>,
            tower::layer::util::Stack<
                SetResponseHeaderLayer<HeaderValue>,
                tower::layer::util::Identity,
            >,
        >,
    >,
> {
    tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
}
```

**Step 2: Apply middleware in main.rs**

```rust
use tower_http::trace::TraceLayer;

let app = axum::Router::new()
    .route("/health", axum::routing::get(health))
    .merge(auth::router())
    .merge(api::router())
    .layer(middleware::security_headers())
    .layer(TraceLayer::new_for_http())
    .with_state(state);
```

**Step 3: Verify compilation**

```bash
cd /home/jan/rice/backend && cargo check
```

**Step 4: Commit**

```bash
git add backend/ && git commit -m "feat: add security headers middleware"
```

---

## Task 7: Frontend Scaffolding

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tsconfig.json`
- Create: `frontend/index.html`
- Create: `frontend/src/main.tsx`
- Create: `frontend/src/App.tsx`
- Create: `frontend/src/globals.css`
- Create: `frontend/src/types/index.ts`
- Create: `frontend/src/lib/api.ts`

**Step 1: Initialize Vite React project**

```bash
cd /home/jan/rice
npm create vite@latest frontend -- --template react-ts
```

**Step 2: Install dependencies**

```bash
cd /home/jan/rice/frontend
npm install react-router-dom
npm install -D @types/react-router-dom
```

**Step 3: Configure vite.config.ts for API proxy**

```typescript
// frontend/vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:3000',
      '/auth': 'http://localhost:3000',
      '/health': 'http://localhost:3000',
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: false,
  },
})
```

**Step 4: Write TypeScript types**

```typescript
// frontend/src/types/index.ts
export interface User {
  id: string
  email: string
  display_name: string
  avatar_url: string | null
  created_at: string
  updated_at: string
}

export interface Trip {
  id: string
  name: string
  destination: string
  start_date: string | null
  end_date: string | null
  cover_image_path: string | null
  created_by: string
  created_at: string
  updated_at: string
  role: string
}

export interface TripMember {
  user_id: string
  email: string
  display_name: string
  avatar_url: string | null
  role: string
  joined_at: string
}

export interface CreateTripRequest {
  name: string
  destination?: string
  start_date?: string
  end_date?: string
}

export interface UpdateTripRequest {
  name?: string
  destination?: string
  start_date?: string
  end_date?: string
}
```

**Step 5: Write API client**

```typescript
// frontend/src/lib/api.ts
import type { User, Trip, TripMember, CreateTripRequest, UpdateTripRequest } from '../types'

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message)
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  })

  if (res.status === 401) {
    window.location.href = '/auth/login'
    throw new ApiError(401, 'Unauthorized')
  }

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: 'Unknown error' }))
    throw new ApiError(res.status, body.error || 'Request failed')
  }

  if (res.status === 204) return undefined as T

  return res.json()
}

export const api = {
  me: () => request<User>('/api/me'),

  trips: {
    list: () => request<Trip[]>('/api/trips'),
    get: (id: string) => request<Trip>(`/api/trips/${id}`),
    create: (data: CreateTripRequest) =>
      request<Trip>('/api/trips', { method: 'POST', body: JSON.stringify(data) }),
    update: (id: string, data: UpdateTripRequest) =>
      request<Trip>(`/api/trips/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
    delete: (id: string) =>
      request<void>(`/api/trips/${id}`, { method: 'DELETE' }),
  },

  members: {
    list: (tripId: string) => request<TripMember[]>(`/api/trips/${tripId}/members`),
    remove: (tripId: string, userId: string) =>
      request<void>(`/api/trips/${tripId}/members/${userId}`, { method: 'DELETE' }),
  },

  invites: {
    create: (tripId: string, email: string, role: string) =>
      request<void>(`/api/trips/${tripId}/invites`, {
        method: 'POST',
        body: JSON.stringify({ email, role }),
      }),
  },

  logout: () => request<void>('/auth/logout', { method: 'POST' }),
}
```

**Step 6: Write globals.css — full design system**

This is the most important file for the visual identity. Write the full CSS design system based on the design doc:

```css
/* frontend/src/globals.css */

/* ============================================
   RICE (旅) — Design System
   Japanese Cyberpunk Aesthetic
   ============================================ */

/* --- Reset --- */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html {
  font-size: 16px;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color-scheme: dark;
}

/* --- Scan Line Texture (root only) --- */
html::after {
  content: '';
  position: fixed;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(255, 255, 255, 0.015) 2px,
    rgba(255, 255, 255, 0.015) 3px
  );
  pointer-events: none;
  z-index: 9999;
}

body {
  font-family: 'IBM Plex Sans', 'Noto Sans JP', system-ui, sans-serif;
  background-color: var(--color-void);
  color: var(--color-text-primary);
  line-height: var(--leading-normal);
  min-height: 100dvh;
}

/* --- Design Tokens --- */
:root {
  /* Backgrounds */
  --color-void: #0a0a0f;
  --color-surface: #0f0f18;
  --color-surface-2: #141420;
  --color-surface-3: #1c1c2e;
  --color-border: #2a2a3d;
  --color-border-2: #3d3d5c;

  /* Neons */
  --color-neon-primary: #ff6b2b;
  --color-neon-red: #ff2d55;
  --color-neon-pink: #ff0080;
  --color-neon-amber: #ffb830;
  --color-neon-cyan: #00d4ff;

  /* Text */
  --color-text-primary: #f0ede8;
  --color-text-secondary: #9896a4;
  --color-text-tertiary: #5c5a6e;
  --color-text-inverse: #0a0a0f;

  /* Glows */
  --glow-primary: 0 0 20px rgba(255, 107, 43, 0.4);
  --glow-pink: 0 0 20px rgba(255, 0, 128, 0.35);
  --glow-cyan: 0 0 20px rgba(0, 212, 255, 0.3);
  --glow-subtle: 0 0 12px rgba(255, 107, 43, 0.15);

  /* Typography */
  --font-sans: 'IBM Plex Sans', 'Noto Sans JP', system-ui, sans-serif;
  --font-mono: 'IBM Plex Mono', 'Courier New', monospace;

  --text-xs: 0.64rem;
  --text-sm: 0.8rem;
  --text-base: 1rem;
  --text-md: 1.25rem;
  --text-lg: 1.563rem;
  --text-xl: 1.953rem;
  --text-2xl: 2.441rem;
  --text-3xl: 3.052rem;

  --leading-tight: 1.2;
  --leading-normal: 1.5;
  --leading-loose: 1.75;

  --tracking-tight: -0.02em;
  --tracking-normal: 0;
  --tracking-wide: 0.08em;
  --tracking-wider: 0.15em;

  /* Spacing (4px grid) */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;
  --space-20: 80px;

  /* Border Radius */
  --radius-sm: 3px;
  --radius-md: 6px;
  --radius-lg: 10px;
  --radius-xl: 16px;
  --radius-full: 9999px;

  /* Shadows */
  --shadow-sm: 0 1px 3px rgba(0,0,0,0.4), 0 1px 2px rgba(0,0,0,0.3);
  --shadow-md: 0 4px 12px rgba(0,0,0,0.5), 0 2px 4px rgba(0,0,0,0.3);
  --shadow-lg: 0 8px 32px rgba(0,0,0,0.6), 0 4px 8px rgba(0,0,0,0.4);
  --shadow-neon-orange: 0 0 0 1px rgba(255,107,43,0.3), 0 0 24px rgba(255,107,43,0.2);
  --shadow-neon-pink: 0 0 0 1px rgba(255,0,128,0.3), 0 0 24px rgba(255,0,128,0.15);
  --shadow-focus: 0 0 0 2px rgba(255,107,43,0.5), inset 0 0 12px rgba(255,107,43,0.05);

  /* Transitions */
  --transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-normal: 250ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-slow: 400ms cubic-bezier(0.4, 0, 0.2, 1);
  --ease-out-quart: cubic-bezier(0.25, 1, 0.5, 1);
  --ease-in-quart: cubic-bezier(0.5, 0, 0.75, 0);

  /* Safe areas */
  --safe-top: env(safe-area-inset-top, 0px);
  --safe-bottom: env(safe-area-inset-bottom, 0px);
}

/* --- Base Typography --- */
h1, h2, h3, h4, h5, h6 {
  letter-spacing: var(--tracking-tight);
  line-height: var(--leading-tight);
  font-weight: 600;
}

h1 { font-size: var(--text-xl); }
h2 { font-size: var(--text-lg); }
h3 { font-size: var(--text-md); }

@media (min-width: 1024px) {
  h1 { font-size: var(--text-2xl); }
  h2 { font-size: var(--text-xl); }
}

a {
  color: var(--color-neon-cyan);
  text-decoration: none;
}

a:hover {
  text-decoration: underline;
}

/* --- HUD Label Style --- */
.label-hud {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wide);
  color: var(--color-text-secondary);
}

/* --- Animations --- */
@keyframes fade-up {
  from { opacity: 0; transform: translateY(8px); }
  to   { opacity: 1; transform: translateY(0); }
}

@keyframes neon-pulse {
  0%, 100% { box-shadow: 0 0 8px rgba(255,107,43,0.3); }
  50%       { box-shadow: 0 0 20px rgba(255,107,43,0.6), 0 0 40px rgba(255,107,43,0.2); }
}

@keyframes scanner {
  0%   { transform: translateY(-100%); }
  100% { transform: translateY(100vh); }
}

.page-enter {
  animation: fade-up 300ms var(--ease-out-quart) both;
}

/* --- Loading Scanner --- */
.loading-scanner::after {
  content: '';
  position: fixed;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, transparent, var(--color-neon-primary), transparent);
  animation: scanner 1.5s linear infinite;
  pointer-events: none;
  z-index: 10000;
}

/* --- Reduced Motion --- */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
  html::after { display: none; }
}

/* --- Scrollbar --- */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: var(--color-void);
}
::-webkit-scrollbar-thumb {
  background: var(--color-border-2);
  border-radius: var(--radius-full);
}

/* --- Selection --- */
::selection {
  background: rgba(255, 107, 43, 0.3);
  color: var(--color-text-primary);
}
```

**Step 7: Write App.tsx with routing**

```tsx
// frontend/src/App.tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { useState, useEffect, createContext, useContext } from 'react'
import type { User } from './types'
import { api } from './lib/api'

interface AuthContextType {
  user: User | null
  loading: boolean
  logout: () => Promise<void>
}

export const AuthContext = createContext<AuthContextType>({
  user: null,
  loading: true,
  logout: async () => {},
})

export function useAuth() {
  return useContext(AuthContext)
}

function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.me()
      .then(setUser)
      .catch(() => setUser(null))
      .finally(() => setLoading(false))
  }, [])

  const logout = async () => {
    await api.logout()
    setUser(null)
    window.location.href = '/auth/login'
  }

  return (
    <AuthContext.Provider value={{ user, loading, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth()

  if (loading) return <div className="loading-scanner" />
  if (!user) {
    window.location.href = '/auth/login'
    return null
  }

  return <>{children}</>
}

// Lazy page imports will be added as pages are built
function Placeholder({ title }: { title: string }) {
  return (
    <div style={{ padding: 'var(--space-10)', color: 'var(--color-text-primary)' }}>
      <h1>{title}</h1>
      <p style={{ color: 'var(--color-text-secondary)' }}>Coming soon</p>
    </div>
  )
}

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<Placeholder title="Login" />} />
          <Route
            path="/"
            element={
              <ProtectedRoute>
                <Placeholder title="Dashboard" />
              </ProtectedRoute>
            }
          />
          <Route
            path="/trips/new"
            element={
              <ProtectedRoute>
                <Placeholder title="New Trip" />
              </ProtectedRoute>
            }
          />
          <Route
            path="/trips/:id"
            element={
              <ProtectedRoute>
                <Placeholder title="Trip Detail" />
              </ProtectedRoute>
            }
          />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  )
}
```

**Step 8: Update main.tsx**

```tsx
// frontend/src/main.tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './globals.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
```

**Step 9: Verify frontend builds**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 10: Commit**

```bash
git add frontend/ && git commit -m "feat: scaffold frontend with Vite, React Router, design system, API client"
```

---

## Task 8: Frontend UI Components

**Files:**
- Create: `frontend/src/components/ui/Button/Button.tsx`
- Create: `frontend/src/components/ui/Button/Button.module.css`
- Create: `frontend/src/components/ui/Input/Input.tsx`
- Create: `frontend/src/components/ui/Input/Input.module.css`
- Create: `frontend/src/components/ui/Card/Card.tsx`
- Create: `frontend/src/components/ui/Card/Card.module.css`
- Create: `frontend/src/components/ui/Badge/Badge.tsx`
- Create: `frontend/src/components/ui/Badge/Badge.module.css`
- Create: `frontend/src/components/ui/Modal/Modal.tsx`
- Create: `frontend/src/components/ui/Modal/Modal.module.css`
- Create: `frontend/src/components/ui/GlowDivider/GlowDivider.tsx`
- Create: `frontend/src/components/ui/GlowDivider/GlowDivider.module.css`

**Implementation notes:** Each component follows this pattern:
- `.tsx` file with a typed props interface accepting `className`
- `.module.css` file using design tokens from `globals.css`
- `data-variant` and `data-size` attributes for state styling

Write all components with the Japanese cyberpunk aesthetic:
- Buttons: sharp corners (radius-md), neon glow on hover, translateY(-1px) lift
- Inputs: bottom-border-only at rest, full border + focus glow on focus
- Cards: surface background, radius-lg, left border accent, corner brackets via pseudo-elements
- Badge: small pill with neon background, inverse text
- Modal: desktop = centered overlay, mobile = bottom sheet
- GlowDivider: thin horizontal rule with 60px neon glow center

**Step 1: Write Button component**

Button.tsx: Accepts `variant` ('primary' | 'secondary' | 'ghost' | 'danger'), `size` ('sm' | 'md' | 'lg'), standard button props.

Button.module.css: Uses neon-pulse animation on primary variant, translateY hover, all transitions on GPU-accelerated properties only.

**Step 2: Write Input component**

Input.tsx: Accepts `label` (renders as HUD label above), `error` (renders below), standard input props.

Input.module.css: Bottom border only at rest (`border-bottom: 1px solid var(--color-border)`), full border + shadow-focus on `:focus`. Label in `font-mono`, uppercase, tracked wide.

**Step 3: Write Card component**

Card.tsx: Wrapper component with optional `active` prop for left border accent.

Card.module.css: `background: var(--color-surface)`, `border-radius: var(--radius-lg)`, `border: 1px solid var(--color-border)`. On hover: `translateY(-2px)` + `shadow-lg` + faint `shadow-neon-orange`. Corner brackets via `::before` and `::after` on the card (small L-shaped pseudo-elements in top-left and bottom-right corners).

**Step 4: Write Badge, Modal, GlowDivider**

Follow the same patterns. Modal uses CSS `backdrop-filter: blur(8px)` behind it.

**Step 5: Verify build**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 6: Commit**

```bash
git add frontend/ && git commit -m "feat: add UI component library (Button, Input, Card, Badge, Modal, GlowDivider)"
```

---

## Task 9: Frontend Layout Components

**Files:**
- Create: `frontend/src/components/layout/AppShell.tsx`
- Create: `frontend/src/components/layout/AppShell.module.css`
- Create: `frontend/src/components/layout/SideNav.tsx`
- Create: `frontend/src/components/layout/SideNav.module.css`
- Create: `frontend/src/components/layout/BottomNav.tsx`
- Create: `frontend/src/components/layout/BottomNav.module.css`
- Create: `frontend/src/components/layout/PageHeader.tsx`
- Create: `frontend/src/components/layout/PageHeader.module.css`
- Create: `frontend/src/hooks/useMediaQuery.ts`

**Step 1: Write useMediaQuery hook**

Simple hook that listens to `window.matchMedia`. Used to switch between SideNav (desktop) and BottomNav (mobile).

```typescript
// frontend/src/hooks/useMediaQuery.ts
import { useState, useEffect } from 'react'

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() =>
    typeof window !== 'undefined' ? window.matchMedia(query).matches : false
  )

  useEffect(() => {
    const mql = window.matchMedia(query)
    const handler = (e: MediaQueryListEvent) => setMatches(e.matches)
    mql.addEventListener('change', handler)
    return () => mql.removeEventListener('change', handler)
  }, [query])

  return matches
}
```

**Step 2: Write SideNav (desktop)**

200px fixed left sidebar. Logo "Rice / 旅" at top in mono font. Navigation items: Trips (home icon), New Trip (+ icon). User avatar + name at bottom with logout.

**Step 3: Write BottomNav (mobile)**

56px fixed bottom tab bar. `backdrop-filter: blur(20px)`, `background: rgba(15,15,24,0.85)`. Tabs: Trips, New Trip, Profile. Safe area padding for iOS.

**Step 4: Write AppShell**

Layout wrapper. Uses `useMediaQuery('(min-width: 1024px)')` to toggle between SideNav and BottomNav. Main content area has appropriate padding for each layout.

**Step 5: Write PageHeader**

Title + optional action button row. Title uses h1, action area right-aligned.

**Step 6: Verify build**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 7: Commit**

```bash
git add frontend/ && git commit -m "feat: add layout components (AppShell, SideNav, BottomNav, PageHeader)"
```

---

## Task 10: Frontend Pages — Login

**Files:**
- Create: `frontend/src/components/auth/LoginScreen.tsx`
- Create: `frontend/src/components/auth/LoginScreen.module.css`
- Create: `frontend/src/pages/LoginPage.tsx`
- Modify: `frontend/src/App.tsx`

**Step 1: Write LoginScreen**

Full-screen centered login page. The star of the aesthetic — this is the first thing users see.

Layout:
- Dark void background with scan lines
- Centered card with the Rice logo "旅" large (text-3xl) in neon primary
- Subtitle "RICE" in mono, tracked wider
- Tagline "plan your journey" in text-secondary
- GlowDivider
- "Sign in with Authentik" button (primary variant, neon pulse animation)
- Clicking the button navigates to `/auth/login`

**Step 2: Write LoginPage**

Wraps LoginScreen. If user is already authenticated, redirect to `/`.

**Step 3: Update App.tsx routes**

Replace the login placeholder with the real `LoginPage`.

**Step 4: Verify build and visually inspect**

```bash
cd /home/jan/rice/frontend && npm run dev
```

**Step 5: Commit**

```bash
git add frontend/ && git commit -m "feat: add login screen with Japanese cyberpunk aesthetic"
```

---

## Task 11: Frontend Pages — Dashboard

**Files:**
- Create: `frontend/src/components/trips/TripCard.tsx`
- Create: `frontend/src/components/trips/TripCard.module.css`
- Create: `frontend/src/components/trips/TripGrid.tsx`
- Create: `frontend/src/components/trips/TripGrid.module.css`
- Create: `frontend/src/hooks/useTrips.ts`
- Create: `frontend/src/pages/DashboardPage.tsx`
- Modify: `frontend/src/App.tsx`

**Step 1: Write useTrips hook**

```typescript
// frontend/src/hooks/useTrips.ts
import { useState, useEffect, useCallback } from 'react'
import type { Trip } from '../types'
import { api } from '../lib/api'

export function useTrips() {
  const [trips, setTrips] = useState<Trip[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    setLoading(true)
    try {
      const data = await api.trips.list()
      setTrips(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load trips')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { refresh() }, [refresh])

  return { trips, loading, error, refresh }
}
```

**Step 2: Write TripCard**

The signature visual component. Shows:
- Cover image (full bleed, with `mix-blend-mode: luminosity` overlay + gradient from bottom)
- Trip name overlaid on the image bottom
- Destination in mono, tracked wide, below image
- Date range (or "No dates" in text-tertiary)
- Role badge (owner/editor/viewer)
- Left border accent: `border-left: 2px solid var(--color-neon-primary)` on hover

If no cover image, show a gradient placeholder using the neon palette.

**Step 3: Write TripGrid**

Responsive CSS Grid: `grid-template-columns: repeat(auto-fill, minmax(260px, 1fr))`, gap `--space-6`.

On mobile (<768px): single column, cards full width.

**Step 4: Write DashboardPage**

Uses AppShell layout. PageHeader with "Your Trips" + "New Trip" button (links to `/trips/new`). If no trips, show empty state with Japanese accent text. If loading, show scanner.

**Step 5: Update App.tsx**

Replace dashboard placeholder with real `DashboardPage`.

**Step 6: Verify build**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 7: Commit**

```bash
git add frontend/ && git commit -m "feat: add dashboard with trip grid and cards"
```

---

## Task 12: Frontend Pages — Trip Detail + Forms

**Files:**
- Create: `frontend/src/components/trips/TripForm.tsx`
- Create: `frontend/src/components/trips/TripForm.module.css`
- Create: `frontend/src/components/trips/TripDetail.tsx`
- Create: `frontend/src/components/trips/TripDetail.module.css`
- Create: `frontend/src/components/trips/CollaboratorList.tsx`
- Create: `frontend/src/components/trips/CollaboratorList.module.css`
- Create: `frontend/src/pages/TripNewPage.tsx`
- Create: `frontend/src/pages/TripDetailPage.tsx`
- Modify: `frontend/src/App.tsx`

**Step 1: Write TripForm**

Used for both create and edit. Fields:
- Name (Input, required)
- Destination (Input)
- Start Date (native date input with custom styling)
- End Date (native date input)
- Submit button

On mobile: full width, single column. On desktop: centered, max-width 560px.

Uses the HUD label style for field labels. Form state managed with `useState`.

**Step 2: Write TripNewPage**

AppShell + PageHeader "New Trip". Contains TripForm. On submit, calls `api.trips.create()` and navigates to the new trip's detail page.

**Step 3: Write CollaboratorList**

List of trip members with:
- Avatar (or initials circle if no avatar)
- Display name
- Email in mono
- Role badge
- Remove button (only shown to owner, not on themselves)

Plus an "Invite" section with email input + role select + send button.

**Step 4: Write TripDetail**

Layout:
- Cover image hero (full width, 200px height, same overlay treatment as TripCard)
- Trip name (h1), destination (mono), date range
- GlowDivider
- Collaborators section (CollaboratorList)
- Edit/Delete buttons (only for editors/owners)

On desktop: two columns (info left, collaborators right). On mobile: stacked.

**Step 5: Write TripDetailPage**

Fetches trip by ID from route params. Shows loading scanner. Handles 404.

**Step 6: Update App.tsx**

Replace remaining placeholders with real pages.

**Step 7: Verify build**

```bash
cd /home/jan/rice/frontend && npm run build
```

**Step 8: Commit**

```bash
git add frontend/ && git commit -m "feat: add trip forms, detail page, and collaborator management UI"
```

---

## Task 13: Cover Image Upload

**Files:**
- Create: `backend/src/api/uploads.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `frontend/src/components/trips/TripForm.tsx`

**Step 1: Write upload handler (backend)**

```rust
// backend/src/api/uploads.rs
use axum::{
    extract::{Multipart, State},
    Json,
};
use std::path::PathBuf;
use tokio::fs;

use crate::{errors::AppError, extractors::TripAccess, AppState};

const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024; // 5MB

pub async fn upload_cover(
    State(state): State<AppState>,
    access: TripAccess,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    access.require_editor()?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        if field.name() != Some("cover") {
            continue;
        }

        let content_type = field.content_type().unwrap_or("").to_string();
        if !["image/jpeg", "image/png", "image/webp"].contains(&content_type.as_str()) {
            return Err(AppError::BadRequest("Only JPEG, PNG, and WebP images are allowed".into()));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read upload: {e}")))?;

        if data.len() > MAX_IMAGE_SIZE {
            return Err(AppError::BadRequest("Image must be under 5MB".into()));
        }

        let ext = match content_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "bin",
        };

        let filename = format!("{}.{ext}", ulid::Ulid::new());
        let dir = PathBuf::from("/data/uploads").join(&access.trip_id);
        fs::create_dir_all(&dir).await.map_err(|e| AppError::Internal(format!("Failed to create upload dir: {e}")))?;

        let path = dir.join(&filename);
        fs::write(&path, &data).await.map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

        let relative_path = format!("/uploads/{}/{filename}", access.trip_id);

        sqlx::query("UPDATE trips SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&relative_path)
            .bind(&access.trip_id)
            .execute(&state.db.write)
            .await?;

        return Ok(Json(serde_json::json!({ "path": relative_path })));
    }

    Err(AppError::BadRequest("No cover image provided".into()))
}
```

**Step 2: Add upload route to API router**

```rust
.route("/api/trips/{trip_id}/cover", post(uploads::upload_cover))
```

**Step 3: Serve uploads directory from Axum**

In `main.rs`, add a static file handler for `/uploads/`:

```rust
use tower_http::services::ServeDir;

let app = axum::Router::new()
    // ... existing routes ...
    .nest_service("/uploads", ServeDir::new("/data/uploads"))
    // ... layers ...
```

**Step 4: Add file input to TripForm (frontend)**

Add an image picker to TripForm. On selection, compress client-side (optional — skip for MVP if bundle size is a concern) and upload via `POST /api/trips/:id/cover` after trip creation.

**Step 5: Verify compilation**

```bash
cd /home/jan/rice/backend && cargo check
cd /home/jan/rice/frontend && npm run build
```

**Step 6: Commit**

```bash
git add backend/ frontend/ && git commit -m "feat: add cover image upload with file serving"
```

---

## Task 14: Static Asset Embedding

**Files:**
- Modify: `backend/src/main.rs`
- Create: `backend/build.rs` (optional, for build-time frontend compilation)

**Step 1: Add rust-embed for serving the SPA**

In `main.rs`, use `rust-embed` to embed the frontend `dist/` directory and serve it as the fallback for all non-API routes:

```rust
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../frontend/dist"]
struct FrontendAssets;

async fn serve_frontend(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try exact path first
    if let Some(file) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
            file.data,
        ).into_response();
    }

    // Fallback to index.html for SPA routing
    match FrontendAssets::get("index.html") {
        Some(file) => (
            [(axum::http::header::CONTENT_TYPE, "text/html")],
            file.data,
        ).into_response(),
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}
```

Add to the router as a fallback:

```rust
let app = axum::Router::new()
    .route("/health", axum::routing::get(health))
    .merge(auth::router())
    .merge(api::router())
    .nest_service("/uploads", ServeDir::new("/data/uploads"))
    .fallback(serve_frontend)
    .layer(middleware::security_headers())
    .layer(TraceLayer::new_for_http())
    .with_state(state);
```

**Step 2: Build frontend first, then backend**

```bash
cd /home/jan/rice/frontend && npm run build
cd /home/jan/rice/backend && cargo build
```

**Step 3: Commit**

```bash
git add backend/ && git commit -m "feat: embed frontend static assets in Rust binary via rust-embed"
```

---

## Task 15: Dockerfile

**Files:**
- Create: `Dockerfile`
- Create: `docker-compose.yml`
- Create: `.dockerignore`

**Step 1: Write .dockerignore**

```
target/
node_modules/
frontend/dist/
.git/
*.md
docs/
```

**Step 2: Write multi-stage Dockerfile**

```dockerfile
# Stage 1: Build frontend
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# Stage 2: Build backend
FROM rust:1.83-bookworm AS backend-builder
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/.sqlx ./backend/.sqlx

# Build deps only (layer caching)
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs
RUN cd backend && SQLX_OFFLINE=true cargo build --release
RUN rm -rf backend/src

# Copy real source + frontend assets
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist
RUN touch backend/src/main.rs
RUN cd backend && SQLX_OFFLINE=true cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates wget && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/backend/target/release/rice /usr/local/bin/rice

RUN mkdir -p /data/db /data/uploads

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget -q --spider http://localhost:3000/health || exit 1

CMD ["rice"]
```

**Step 3: Write docker-compose.yml**

Use the docker-compose.yml from the design doc, adding SMTP env vars.

**Step 4: Test Docker build locally**

```bash
cd /home/jan/rice && docker build -t rice:dev .
```

**Step 5: Commit**

```bash
git add Dockerfile docker-compose.yml .dockerignore && git commit -m "feat: add Dockerfile and docker-compose for single-container deployment"
```

---

## Task 16: Integration Testing

**Files:**
- Create: `backend/tests/common/mod.rs`
- Create: `backend/tests/api_trips_test.rs`

**Step 1: Write test helpers**

```rust
// backend/tests/common/mod.rs
use sqlx::SqlitePool;

/// Creates an in-memory SQLite database with migrations applied
pub async fn test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

/// Inserts a test user and returns their ID
pub async fn create_test_user(pool: &SqlitePool, email: &str) -> String {
    let id = ulid::Ulid::new().to_string();
    sqlx::query("INSERT INTO users (id, email, display_name) VALUES (?1, ?2, ?3)")
        .bind(&id)
        .bind(email)
        .bind("Test User")
        .execute(pool)
        .await
        .unwrap();
    id
}
```

**Step 2: Write trip CRUD tests**

Test the core domain operations directly against the database:
- Creating a trip adds the creator as owner
- Listing trips only shows trips where user is a member
- Updating a trip requires editor role
- Deleting a trip cascades to members and invites

**Step 3: Run tests**

```bash
cd /home/jan/rice/backend && cargo test
```

**Step 4: Commit**

```bash
git add backend/tests/ && git commit -m "test: add integration tests for trip CRUD and access control"
```

---

## Task 17: Final Wiring + Polish

**Step 1: Review all imports and module declarations**

Ensure `main.rs` has all `mod` declarations, all routers are merged, all types are exported correctly.

**Step 2: Run full build (frontend + backend)**

```bash
cd /home/jan/rice/frontend && npm run build
cd /home/jan/rice/backend && cargo build --release
```

**Step 3: Run all tests**

```bash
cd /home/jan/rice/backend && cargo test
```

**Step 4: Test Docker build**

```bash
cd /home/jan/rice && docker build -t rice:dev .
```

**Step 5: Create .env.example**

```
DATABASE_URL=sqlite:///data/db/rice.db
JWT_SECRET=change-me-to-a-random-32-byte-string
AUTHENTIK_CLIENT_ID=
AUTHENTIK_CLIENT_SECRET=
AUTHENTIK_BASE_URL=https://auth.yourdomain.com
APP_BASE_URL=https://rice.yourdomain.com
SMTP_HOST=
SMTP_PORT=587
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_FROM=rice@yourdomain.com
```

**Step 6: Final commit**

```bash
git add -A && git commit -m "feat: complete Rice v1 MVP — auth, trips, invites, cyberpunk UI"
```

---

## Dependency Graph

```
Task 1  (Rust scaffold)
  └→ Task 2  (Database)
       └→ Task 3  (Auth + JWT + extractors)
            ├→ Task 4  (Trip CRUD API)
            │    └→ Task 5  (Members + Invites + Email)
            │         └→ Task 13 (Cover image upload)
            └→ Task 6  (Security middleware)

Task 7  (Frontend scaffold)
  └→ Task 8  (UI components)
       └→ Task 9  (Layout components)
            ├→ Task 10 (Login page)
            ├→ Task 11 (Dashboard)
            └→ Task 12 (Trip detail + forms)

Task 13 + Task 12 → Task 14 (Static asset embedding)
Task 14 → Task 15 (Dockerfile)
Task 4 + Task 5 → Task 16 (Integration tests)
All → Task 17 (Final wiring)
```

Backend tasks (1-6, 13) and frontend tasks (7-12) can be **worked in parallel** after Task 2 is complete, since the frontend uses the API client that doesn't depend on the backend being compiled.
