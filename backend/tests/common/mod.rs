use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

pub async fn test_db() -> SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(":memory:")
        .pragma("foreign_keys", "ON");
    // Use max_connections(1) so all queries hit the same in-memory database.
    // SQLite :memory: databases are per-connection; with >1 connection each
    // would get its own empty database.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

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
