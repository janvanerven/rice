mod common;

/// Helper to create a trip and add the user as owner
async fn create_trip_with_owner(pool: &sqlx::SqlitePool, user_id: &str, name: &str) -> String {
    let trip_id = ulid::Ulid::new().to_string();
    sqlx::query("INSERT INTO trips (id, name, destination, created_by) VALUES (?1, ?2, '', ?3)")
        .bind(&trip_id)
        .bind(name)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')")
        .bind(&trip_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    trip_id
}

/// Helper to insert an accommodation directly
async fn insert_accommodation(
    pool: &sqlx::SqlitePool,
    trip_id: &str,
    name: &str,
    check_in: Option<&str>,
    check_out: Option<&str>,
) -> String {
    let id = ulid::Ulid::new().to_string();
    sqlx::query(
        "INSERT INTO accommodations (id, trip_id, name, check_in, check_out) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&id)
    .bind(trip_id)
    .bind(name)
    .bind(check_in)
    .bind(check_out)
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn test_accommodation_crud() {
    let pool = common::test_db().await;
    let user_id = common::create_test_user(&pool, "test@example.com").await;
    let trip_id = create_trip_with_owner(&pool, &user_id, "Japan Trip").await;

    // Create
    let acc_id = insert_accommodation(&pool, &trip_id, "Hotel Akira", Some("2026-03-01"), Some("2026-03-05")).await;

    let row: (String, String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT name, trip_id, check_in, check_out FROM accommodations WHERE id = ?1",
    )
    .bind(&acc_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "Hotel Akira");
    assert_eq!(row.1, trip_id);
    assert_eq!(row.2.as_deref(), Some("2026-03-01"));
    assert_eq!(row.3.as_deref(), Some("2026-03-05"));

    // Update
    sqlx::query(
        "UPDATE accommodations SET name = ?1, address = ?2, updated_at = datetime('now') WHERE id = ?3",
    )
    .bind("Hotel Neo-Tokyo")
    .bind("1-2-3 Shibuya")
    .bind(&acc_id)
    .execute(&pool)
    .await
    .unwrap();

    let updated: (String, Option<String>) = sqlx::query_as(
        "SELECT name, address FROM accommodations WHERE id = ?1",
    )
    .bind(&acc_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(updated.0, "Hotel Neo-Tokyo");
    assert_eq!(updated.1.as_deref(), Some("1-2-3 Shibuya"));

    // Delete
    sqlx::query("DELETE FROM accommodations WHERE id = ?1")
        .bind(&acc_id)
        .execute(&pool)
        .await
        .unwrap();

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM accommodations WHERE id = ?1")
            .bind(&acc_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_trip_delete_cascades_accommodations() {
    let pool = common::test_db().await;
    let user_id = common::create_test_user(&pool, "test@example.com").await;
    let trip_id = create_trip_with_owner(&pool, &user_id, "Cascade Trip").await;

    insert_accommodation(&pool, &trip_id, "Hotel A", None, None).await;
    insert_accommodation(&pool, &trip_id, "Hotel B", None, None).await;

    // Verify 2 accommodations exist
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM accommodations WHERE trip_id = ?1")
            .bind(&trip_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);

    // Delete the trip
    sqlx::query("DELETE FROM trips WHERE id = ?1")
        .bind(&trip_id)
        .execute(&pool)
        .await
        .unwrap();

    // Accommodations should be cascade-deleted
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM accommodations WHERE trip_id = ?1")
            .bind(&trip_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_accommodations_cross_trip_isolation() {
    let pool = common::test_db().await;
    let user1 = common::create_test_user(&pool, "user1@example.com").await;
    let user2 = common::create_test_user(&pool, "user2@example.com").await;

    let trip1 = create_trip_with_owner(&pool, &user1, "Trip 1").await;
    let trip2 = create_trip_with_owner(&pool, &user2, "Trip 2").await;

    insert_accommodation(&pool, &trip1, "Hotel Alpha", Some("2026-03-01"), None).await;
    insert_accommodation(&pool, &trip2, "Hotel Beta", Some("2026-04-01"), None).await;

    // Trip 1 should only see Hotel Alpha
    let trip1_accs: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM accommodations WHERE trip_id = ?1",
    )
    .bind(&trip1)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(trip1_accs.len(), 1);
    assert_eq!(trip1_accs[0].0, "Hotel Alpha");

    // Trip 2 should only see Hotel Beta
    let trip2_accs: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM accommodations WHERE trip_id = ?1",
    )
    .bind(&trip2)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(trip2_accs.len(), 1);
    assert_eq!(trip2_accs[0].0, "Hotel Beta");
}

#[tokio::test]
async fn test_accommodation_ordering_nulls_last() {
    let pool = common::test_db().await;
    let user_id = common::create_test_user(&pool, "test@example.com").await;
    let trip_id = create_trip_with_owner(&pool, &user_id, "Order Trip").await;

    // Insert in deliberately wrong order: undated, late, early
    insert_accommodation(&pool, &trip_id, "No Date Hotel", None, None).await;
    insert_accommodation(&pool, &trip_id, "Late Hotel", Some("2026-03-10"), None).await;
    insert_accommodation(&pool, &trip_id, "Early Hotel", Some("2026-03-01"), None).await;

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT name, check_in FROM accommodations WHERE trip_id = ?1 \
         ORDER BY check_in IS NULL ASC, check_in ASC, created_at ASC",
    )
    .bind(&trip_id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, "Early Hotel");
    assert_eq!(rows[1].0, "Late Hotel");
    assert_eq!(rows[2].0, "No Date Hotel"); // nulls last
}
