mod api;
mod auth;
mod config;
mod db;
mod email;
mod errors;
mod extractors;
mod middleware;
mod models;

use config::Config;
use db::DbPools;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbPools>,
    pub email: Option<Arc<email::EmailService>>,
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

    tracing::info!("Rice starting on {}:{}", config.host, config.port);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .expect("Failed to bind address");

    let (xfo, xcto, xxss) = middleware::security_headers();

    let upload_dir =
        std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/data/uploads".into());

    let app = axum::Router::new()
        .route("/health", axum::routing::get(health))
        .merge(auth::router())
        .merge(api::router())
        .nest_service("/uploads", tower_http::services::ServeDir::new(&upload_dir))
        .layer(xfo)
        .layer(xcto)
        .layer(xxss)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    axum::serve(listener, app).await.expect("Server failed");
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<&'static str, errors::AppError> {
    sqlx::query("SELECT 1").execute(&state.db.read).await?;
    Ok("ok")
}
