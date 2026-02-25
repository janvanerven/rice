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
