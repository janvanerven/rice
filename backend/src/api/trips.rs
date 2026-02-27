use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::{
    errors::AppError,
    extractors::{AuthUser, TripAccess},
    models::{Attribution, CreateTripRequest, Trip, TripWithRoleAndAttribution, UpdateTripRequest},
    AppState,
};

pub async fn list_trips(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<TripWithRoleAndAttribution>>, AppError> {
    let rows: Vec<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
        String,
    )> = sqlx::query_as(
        "SELECT t.id, t.name, t.destination, t.start_date, t.end_date, \
         t.cover_image_path, t.created_by, t.created_at, t.updated_at, tm.role \
         FROM trips t \
         JOIN trip_members tm ON tm.trip_id = t.id \
         WHERE tm.user_id = ?1 \
         ORDER BY t.created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.db.read)
    .await?;

    // Batch-load all trip attributions (small table, no dynamic SQL needed)
    let all_attrs: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT entity_id, author_name, author_url, source_url \
         FROM image_attributions WHERE entity_type = 'trip'",
    )
    .fetch_all(&state.db.read)
    .await?;

    let attr_map: std::collections::HashMap<String, Attribution> = all_attrs
        .into_iter()
        .map(|(id, name, url, source)| {
            (id, Attribution {
                author_name: name,
                author_url: url,
                source_url: source,
            })
        })
        .collect();

    let trips = rows
        .into_iter()
        .map(|row| {
            let id = row.0.clone();
            TripWithRoleAndAttribution {
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
                attribution: attr_map.get(&id).cloned(),
            }
        })
        .collect();

    Ok(Json(trips))
}

pub async fn create_trip(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateTripRequest>,
) -> Result<(StatusCode, Json<Trip>), AppError> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Trip name cannot be empty".into()));
    }

    let trip_id = ulid::Ulid::new().to_string();
    let destination = req.destination.unwrap_or_default();

    let mut tx = state.db.write.begin().await?;

    sqlx::query(
        "INSERT INTO trips (id, name, destination, start_date, end_date, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&trip_id)
    .bind(&name)
    .bind(&destination)
    .bind(&req.start_date)
    .bind(&req.end_date)
    .bind(&user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO trip_members (trip_id, user_id, role) VALUES (?1, ?2, 'owner')",
    )
    .bind(&trip_id)
    .bind(&user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&trip_id)
        .fetch_one(&state.db.read)
        .await?;

    Ok((StatusCode::CREATED, Json(trip)))
}

pub async fn get_trip(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<TripWithRoleAndAttribution>, AppError> {
    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    let attribution: Option<(String, String, String)> = sqlx::query_as(
        "SELECT author_name, author_url, source_url FROM image_attributions \
         WHERE entity_type = 'trip' AND entity_id = ?1",
    )
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?;

    Ok(Json(TripWithRoleAndAttribution {
        trip,
        role: access.role,
        attribution: attribution.map(|(name, url, source)| Attribution {
            author_name: name,
            author_url: url,
            source_url: source,
        }),
    }))
}

pub async fn update_trip(
    State(state): State<AppState>,
    access: TripAccess,
    Json(req): Json<UpdateTripRequest>,
) -> Result<Json<Trip>, AppError> {
    access.require_editor()?;

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    let name = req.name.unwrap_or(trip.name);
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("Trip name cannot be empty".into()));
    }
    let destination = req.destination.unwrap_or(trip.destination);
    let start_date = req.start_date.or(trip.start_date);
    let end_date = req.end_date.or(trip.end_date);

    sqlx::query(
        "UPDATE trips SET name = ?1, destination = ?2, start_date = ?3, \
         end_date = ?4, updated_at = datetime('now') WHERE id = ?5",
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
