use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{
    errors::AppError,
    extractors::TripAccess,
    models::{Accommodation, CreateAccommodationRequest, UpdateAccommodationRequest},
    AppState,
};

#[derive(Deserialize)]
pub struct AccommodationPath {
    pub trip_id: String,
    pub accommodation_id: String,
}

pub async fn list_accommodations(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<Vec<Accommodation>>, AppError> {
    let rows: Vec<Accommodation> = sqlx::query_as(
        "SELECT * FROM accommodations WHERE trip_id = ?1 \
         ORDER BY check_in IS NULL ASC, check_in ASC, created_at ASC",
    )
    .bind(&access.trip_id)
    .fetch_all(&state.db.read)
    .await?;

    Ok(Json(rows))
}

pub async fn create_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Json(req): Json<CreateAccommodationRequest>,
) -> Result<(StatusCode, Json<Accommodation>), AppError> {
    access.require_editor()?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Accommodation name cannot be empty".into()));
    }

    let id = ulid::Ulid::new().to_string();

    sqlx::query(
        "INSERT INTO accommodations (id, trip_id, name, address, check_in, check_out, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(&access.trip_id)
    .bind(&name)
    .bind(&req.address)
    .bind(&req.check_in)
    .bind(&req.check_out)
    .bind(&req.notes)
    .execute(&state.db.write)
    .await?;

    let accommodation: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1",
    )
    .bind(&id)
    .fetch_one(&state.db.read)
    .await?;

    Ok((StatusCode::CREATED, Json(accommodation)))
}

pub async fn update_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Path(path): Path<AccommodationPath>,
    Json(req): Json<UpdateAccommodationRequest>,
) -> Result<Json<Accommodation>, AppError> {
    access.require_editor()?;

    let existing: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(&path.accommodation_id)
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?
    .ok_or_else(|| AppError::NotFound("Accommodation not found".into()))?;

    let name = req.name.unwrap_or(existing.name);
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Accommodation name cannot be empty".into()));
    }
    // Note: None means "keep existing" — clients cannot clear optional fields to null
    let address = req.address.or(existing.address);
    let check_in = req.check_in.or(existing.check_in);
    let check_out = req.check_out.or(existing.check_out);
    let notes = req.notes.or(existing.notes);

    sqlx::query(
        "UPDATE accommodations SET name = ?1, address = ?2, check_in = ?3, \
         check_out = ?4, notes = ?5, updated_at = datetime('now') WHERE id = ?6",
    )
    .bind(&name)
    .bind(&address)
    .bind(&check_in)
    .bind(&check_out)
    .bind(&notes)
    .bind(&path.accommodation_id)
    .execute(&state.db.write)
    .await?;

    let updated: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1",
    )
    .bind(&path.accommodation_id)
    .fetch_one(&state.db.read)
    .await?;

    Ok(Json(updated))
}

pub async fn delete_accommodation(
    State(state): State<AppState>,
    access: TripAccess,
    Path(path): Path<AccommodationPath>,
) -> Result<StatusCode, AppError> {
    access.require_editor()?;

    let result = sqlx::query(
        "DELETE FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(&path.accommodation_id)
    .bind(&access.trip_id)
    .execute(&state.db.write)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Accommodation not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn upload_accommodation_cover(
    State(state): State<AppState>,
    access: TripAccess,
    Path(path): Path<AccommodationPath>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    access.require_editor()?;

    // Verify accommodation belongs to this trip
    let _existing: Accommodation = sqlx::query_as(
        "SELECT * FROM accommodations WHERE id = ?1 AND trip_id = ?2",
    )
    .bind(&path.accommodation_id)
    .bind(&access.trip_id)
    .fetch_optional(&state.db.read)
    .await?
    .ok_or_else(|| AppError::NotFound("Accommodation not found".into()))?;

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
            return Err(AppError::BadRequest(
                "Only JPEG, PNG, and WebP images allowed".into(),
            ));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read upload: {e}")))?;

        if data.len() > 5 * 1024 * 1024 {
            return Err(AppError::BadRequest("Image must be under 5MB".into()));
        }

        let ext = match content_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "bin",
        };

        let filename = format!("{}.{ext}", ulid::Ulid::new());
        let upload_base = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/data/uploads".into());
        let dir = std::path::PathBuf::from(&upload_base).join(&access.trip_id);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create upload dir: {e}")))?;

        let file_path = dir.join(&filename);
        tokio::fs::write(&file_path, &data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

        let relative_path = format!("/uploads/{}/{filename}", access.trip_id);

        sqlx::query(
            "UPDATE accommodations SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&relative_path)
        .bind(&path.accommodation_id)
        .execute(&state.db.write)
        .await?;

        return Ok(Json(serde_json::json!({ "path": relative_path })));
    }

    Err(AppError::BadRequest("No cover image provided".into()))
}
