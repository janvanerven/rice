use axum::{extract::{Multipart, State}, Json};
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
            return Err(AppError::BadRequest(
                "Only JPEG, PNG, and WebP images allowed".into(),
            ));
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
        let upload_base = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/data/uploads".into());
        let dir = PathBuf::from(&upload_base).join(&access.trip_id);
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create upload dir: {e}")))?;

        let path = dir.join(&filename);
        fs::write(&path, &data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

        let relative_path = format!("/uploads/{}/{filename}", access.trip_id);

        sqlx::query(
            "UPDATE trips SET cover_image_path = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&relative_path)
        .bind(&access.trip_id)
        .execute(&state.db.write)
        .await?;

        return Ok(Json(serde_json::json!({ "path": relative_path })));
    }

    Err(AppError::BadRequest("No cover image provided".into()))
}
