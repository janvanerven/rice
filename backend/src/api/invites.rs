use axum::{extract::State, http::StatusCode, Json};
use sha2::{Digest, Sha256};

use crate::{errors::AppError, extractors::TripAccess, models::CreateInviteRequest, AppState};

pub async fn create_invite(
    State(state): State<AppState>,
    access: TripAccess,
    Json(req): Json<CreateInviteRequest>,
) -> Result<StatusCode, AppError> {
    access.require_owner()?;

    if !["editor", "viewer"].contains(&req.role.as_str()) {
        return Err(AppError::BadRequest(
            "Role must be 'editor' or 'viewer'".into(),
        ));
    }

    // Check if user is already a member
    let already_member: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM trip_members tm
         JOIN users u ON u.id = tm.user_id
         WHERE tm.trip_id = ?1 AND u.email = ?2",
    )
    .bind(&access.trip_id)
    .bind(&req.email)
    .fetch_one(&state.db.read)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if already_member {
        return Err(AppError::BadRequest(
            "User is already a member of this trip".into(),
        ));
    }

    // Check for existing unclaimed invite
    let existing_invite: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM invites WHERE trip_id = ?1 AND email = ?2 AND claimed_by IS NULL",
    )
    .bind(&access.trip_id)
    .bind(&req.email)
    .fetch_one(&state.db.read)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if existing_invite {
        return Err(AppError::BadRequest(
            "An invite for this email is already pending".into(),
        ));
    }

    // Generate token using OsRng (consistent with auth module)
    let token: String = {
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut bytes);
        hex::encode(bytes)
    };
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let expires_at = chrono::Utc::now() + chrono::TimeDelta::days(7);
    let id = ulid::Ulid::new().to_string();

    let trip_name: String = sqlx::query_scalar("SELECT name FROM trips WHERE id = ?1")
        .bind(&access.trip_id)
        .fetch_one(&state.db.read)
        .await?;

    sqlx::query(
        "INSERT INTO invites (id, trip_id, email, token_hash, role, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&id)
    .bind(&access.trip_id)
    .bind(&req.email)
    .bind(&token_hash)
    .bind(&req.role)
    .bind(expires_at.to_rfc3339())
    .execute(&state.db.write)
    .await?;

    // Send invite email
    let invite_url = format!(
        "{}/invite?token={}",
        state.config.app_base_url.trim_end_matches('/'),
        token
    );

    if let Some(ref email_service) = state.email {
        if let Err(e) = email_service
            .send_invite(&req.email, &trip_name, &access.user.display_name, &invite_url)
            .await
        {
            tracing::warn!("Failed to send invite email: {e}");
            // Don't fail the request -- invite is created, email is best-effort
        }
    }

    Ok(StatusCode::CREATED)
}
