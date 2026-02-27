use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    errors::AppError,
    extractors::TripAccess,
    models::{Accommodation, Attribution, AutoCoverResponse, Trip},
    services::unsplash,
    AppState,
};

use super::accommodations::AccommodationPath;

/// POST /api/trips/{trip_id}/auto-cover
pub async fn auto_cover_trip(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<AutoCoverResponse>, AppError> {
    access.require_editor()?;

    let access_key = state
        .config
        .unsplash_access_key
        .as_deref()
        .ok_or_else(|| AppError::ServiceUnavailable("Auto-cover service not configured".into()))?;

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    if trip.cover_image_path.is_some() {
        return Err(AppError::BadRequest("Trip already has a cover image".into()));
    }

    check_rate_limit(&state, "trip", &access.trip_id).await?;

    let query = trip.destination.trim().to_string();
    if query.is_empty() {
        return Err(AppError::NotFound(
            "No destination set for image search".into(),
        ));
    }

    let client = &state.http_client;
    let photo = unsplash::search(client, access_key, &query)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("No matching image found".into()))?;

    let dir = std::path::PathBuf::from(&state.config.upload_dir).join(&access.trip_id);
    let (tmp_path, filename) = unsplash::download(client, &photo, &dir)
        .await
        .map_err(AppError::Internal)?;

    let relative_path = format!("/{}/{filename}", access.trip_id);

    let mut tx = state.db.write.begin().await?;

    // Race protection: double-check cover is still null
    let still_null: bool =
        sqlx::query_scalar("SELECT cover_image_path IS NULL FROM trips WHERE id = ?1")
            .bind(&access.trip_id)
            .fetch_one(&mut *tx)
            .await?;

    if !still_null {
        drop(tx);
        unsplash::cleanup_temp(&tmp_path).await;
        return Err(AppError::BadRequest(
            "Trip already has a cover image".into(),
        ));
    }

    sqlx::query(
        "UPDATE trips SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2",
    )
    .bind(&relative_path)
    .bind(&access.trip_id)
    .execute(&mut *tx)
    .await?;

    upsert_attribution(&mut tx, "trip", &access.trip_id, &photo).await?;

    tx.commit().await?;

    if let Err(e) = unsplash::finalize(&tmp_path, &filename).await {
        tracing::error!("Failed to finalize auto-cover file: {e}");
    }

    unsplash::track_download(client.clone(), access_key.to_string(), &photo);

    Ok(Json(AutoCoverResponse {
        path: relative_path,
        attribution: Attribution {
            author_name: photo.user.name,
            author_url: photo.user.links.html,
            source_url: photo.links.html,
        },
    }))
}

/// POST /api/trips/{trip_id}/accommodations/{accommodation_id}/auto-cover
pub async fn auto_cover_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Path(path): Path<AccommodationPath>,
) -> Result<Json<AutoCoverResponse>, AppError> {
    access.require_editor()?;

    let access_key = state
        .config
        .unsplash_access_key
        .as_deref()
        .ok_or_else(|| AppError::ServiceUnavailable("Auto-cover service not configured".into()))?;

    let acc: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(&path.accommodation_id)
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?
    .ok_or_else(|| AppError::NotFound("Accommodation not found".into()))?;

    if acc.cover_image_path.is_some() {
        return Err(AppError::BadRequest(
            "Accommodation already has a cover image".into(),
        ));
    }

    check_rate_limit(&state, "accommodation", &path.accommodation_id).await?;

    let trip: Trip = sqlx::query_as("SELECT * FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    let client = &state.http_client;
    let queries = build_accommodation_queries(&acc, &trip);
    let mut photo = None;

    for query in &queries {
        if query.is_empty() {
            continue;
        }
        match unsplash::search(client, access_key, query).await {
            Ok(Some(p)) => {
                photo = Some(p);
                break;
            }
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!("Unsplash search failed for '{query}': {e}");
                continue;
            }
        }
    }

    let photo = photo.ok_or_else(|| AppError::NotFound("No matching image found".into()))?;

    let dir = std::path::PathBuf::from(&state.config.upload_dir).join(&access.trip_id);
    let (tmp_path, filename) = unsplash::download(client, &photo, &dir)
        .await
        .map_err(AppError::Internal)?;

    let relative_path = format!("/{}/{filename}", access.trip_id);

    let mut tx = state.db.write.begin().await?;

    // Race protection: double-check cover is still null
    let still_null: bool = sqlx::query_scalar(
        "SELECT cover_image_path IS NULL FROM accommodations WHERE id = ?1",
    )
    .bind(&path.accommodation_id)
    .fetch_one(&mut *tx)
    .await?;

    if !still_null {
        drop(tx);
        unsplash::cleanup_temp(&tmp_path).await;
        return Err(AppError::BadRequest(
            "Accommodation already has a cover image".into(),
        ));
    }

    sqlx::query(
        "UPDATE accommodations SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2",
    )
    .bind(&relative_path)
    .bind(&path.accommodation_id)
    .execute(&mut *tx)
    .await?;

    upsert_attribution(&mut tx, "accommodation", &path.accommodation_id, &photo).await?;

    tx.commit().await?;

    if let Err(e) = unsplash::finalize(&tmp_path, &filename).await {
        tracing::error!("Failed to finalize auto-cover file: {e}");
    }

    unsplash::track_download(client.clone(), access_key.to_string(), &photo);

    Ok(Json(AutoCoverResponse {
        path: relative_path,
        attribution: Attribution {
            author_name: photo.user.name,
            author_url: photo.user.links.html,
            source_url: photo.links.html,
        },
    }))
}

fn build_accommodation_queries(acc: &Accommodation, trip: &Trip) -> Vec<String> {
    let mut queries = Vec::new();

    let name = acc.name.trim().to_string();
    if !name.is_empty() {
        queries.push(name.clone());
    }

    if let Some(addr) = &acc.address {
        let combined = format!("{} {}", name, addr.trim());
        if combined.trim().len() > name.len() {
            queries.push(combined);
        }
    }

    let dest = trip.destination.trim().to_string();
    if !dest.is_empty() {
        queries.push(dest);
    }

    queries
}

async fn check_rate_limit(
    state: &AppState,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), AppError> {
    let recent: Option<String> = sqlx::query_scalar(
        "SELECT fetched_at FROM image_attributions \
         WHERE entity_type = ?1 AND entity_id = ?2 \
         AND fetched_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-10 minutes')",
    )
    .bind(entity_type)
    .bind(entity_id)
    .fetch_optional(&state.db.read)
    .await?;

    if recent.is_some() {
        return Err(AppError::BadRequest(
            "Auto-cover was attempted recently, please wait".into(),
        ));
    }

    Ok(())
}

async fn upsert_attribution(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    entity_type: &str,
    entity_id: &str,
    photo: &unsplash::UnsplashPhoto,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO image_attributions (entity_type, entity_id, author_name, author_url, source_url) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(entity_type, entity_id) DO UPDATE SET \
         author_name = excluded.author_name, author_url = excluded.author_url, \
         source_url = excluded.source_url, fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(&photo.user.name)
    .bind(&photo.user.links.html)
    .bind(&photo.links.html)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
