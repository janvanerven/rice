mod auth;
mod config;
mod db;
mod errors;
mod extractors;
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

    let db = DbPools::connect(&config.database_url)
        .await
        .unwrap_or_else(|err| {
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
        .merge(auth::router())
        .with_state(state);

    axum::serve(listener, app).await.expect("Server failed");
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<&'static str, errors::AppError> {
    sqlx::query("SELECT 1").execute(&state.db.read).await?;
    Ok("ok")
}
