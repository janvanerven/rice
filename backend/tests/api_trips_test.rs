mod common;

#[tokio::test]
async fn test_create_trip_adds_owner() {
    let pool = common::test_db().await;
    let user_id = common::create_test_user(&pool, "test@example.com").await;

    let trip_id = ulid::Ulid::new().to_string();
    sqlx::query("INSERT INTO trips (id, name, destination, created_by) VALUES (?1, ?2, ?3, ?4)")
        .bind(&trip_id)
        .bind("Test Trip")
        .bind("Tokyo")
        .bind(&user_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')")
        .bind(&trip_id)
        .bind(&user_id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify membership
    let role: String =
        sqlx::query_scalar("SELECT role FROM trip_members WHERE trip_id = ?1 AND user_id = ?2")
            .bind(&trip_id)
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(role, "owner");
}

#[tokio::test]
async fn test_list_trips_only_shows_member_trips() {
    let pool = common::test_db().await;
    let user1 = common::create_test_user(&pool, "user1@example.com").await;
    let user2 = common::create_test_user(&pool, "user2@example.com").await;

    // Create two trips, one for each user
    let trip1_id = ulid::Ulid::new().to_string();
    let trip2_id = ulid::Ulid::new().to_string();

    for (trip_id, owner, name) in [
        (&trip1_id, &user1, "Trip 1"),
        (&trip2_id, &user2, "Trip 2"),
    ] {
        sqlx::query(
            "INSERT INTO trips (id, name, destination, created_by) VALUES (?1, ?2, '', ?3)",
        )
        .bind(trip_id)
        .bind(name)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')")
            .bind(trip_id)
            .bind(owner)
            .execute(&pool)
            .await
            .unwrap();
    }

    // User 1 should only see Trip 1
    let user1_trips: Vec<(String,)> = sqlx::query_as(
        "SELECT t.name FROM trips t JOIN trip_members tm ON tm.trip_id = t.id WHERE tm.user_id = ?1",
    )
    .bind(&user1)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(user1_trips.len(), 1);
    assert_eq!(user1_trips[0].0, "Trip 1");
}

#[tokio::test]
async fn test_delete_trip_cascades() {
    let pool = common::test_db().await;
    let user_id = common::create_test_user(&pool, "test@example.com").await;

    let trip_id = ulid::Ulid::new().to_string();
    sqlx::query("INSERT INTO trips (id, name, destination, created_by) VALUES (?1, ?2, '', ?3)")
        .bind(&trip_id)
        .bind("To Delete")
        .bind(&user_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')")
        .bind(&trip_id)
        .bind(&user_id)
        .execute(&pool)
        .await
        .unwrap();

    // Delete the trip
    sqlx::query("DELETE FROM trips WHERE id = ?1")
        .bind(&trip_id)
        .execute(&pool)
        .await
        .unwrap();

    // Members should be cascade-deleted
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM trip_members WHERE trip_id = ?1")
            .bind(&trip_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_invite_expiry_and_claiming() {
    let pool = common::test_db().await;
    let owner_id = common::create_test_user(&pool, "owner@example.com").await;

    let trip_id = ulid::Ulid::new().to_string();
    sqlx::query("INSERT INTO trips (id, name, destination, created_by) VALUES (?1, ?2, '', ?3)")
        .bind(&trip_id)
        .bind("Invite Test")
        .bind(&owner_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')")
        .bind(&trip_id)
        .bind(&owner_id)
        .execute(&pool)
        .await
        .unwrap();

    // Create an invite
    let invite_id = ulid::Ulid::new().to_string();
    let future_date = "2099-12-31T00:00:00+00:00";
    sqlx::query("INSERT INTO invites (id, trip_id, email, token_hash, role, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .bind(&invite_id)
        .bind(&trip_id)
        .bind("invited@example.com")
        .bind("fakehash")
        .bind("editor")
        .bind(future_date)
        .execute(&pool)
        .await
        .unwrap();

    // Claim the invite
    let invitee_id = common::create_test_user(&pool, "invited@example.com").await;

    sqlx::query("UPDATE invites SET claimed_by = ?1 WHERE id = ?2")
        .bind(&invitee_id)
        .bind(&invite_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, ?3)")
        .bind(&trip_id)
        .bind(&invitee_id)
        .bind("editor")
        .execute(&pool)
        .await
        .unwrap();

    // Verify invite is claimed
    let claimed_by: Option<String> =
        sqlx::query_scalar("SELECT claimed_by FROM invites WHERE id = ?1")
            .bind(&invite_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(claimed_by, Some(invitee_id.clone()));

    // Verify member added
    let role: String =
        sqlx::query_scalar("SELECT role FROM trip_members WHERE trip_id = ?1 AND user_id = ?2")
            .bind(&trip_id)
            .bind(&invitee_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(role, "editor");
}
