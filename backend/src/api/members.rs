use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{errors::AppError, extractors::TripAccess, models::MemberResponse, AppState};

pub async fn list_members(
    State(state): State<AppState>,
    access: TripAccess,
) -> Result<Json<Vec<MemberResponse>>, AppError> {
    let members: Vec<MemberResponse> = sqlx::query_as::<_, (String, String, String, Option<String>, String, String)>(
        "SELECT u.id, u.email, u.display_name, u.avatar_url, tm.role, tm.joined_at
         FROM trip_members tm
         JOIN users u ON u.id = tm.user_id
         WHERE tm.trip_id = ?1
         ORDER BY tm.joined_at"
    )
    .bind(&access.trip_id)
    .fetch_all(&state.db.read)
    .await?
    .into_iter()
    .map(|row| MemberResponse {
        user_id: row.0,
        email: row.1,
        display_name: row.2,
        avatar_url: row.3,
        role: row.4,
        joined_at: row.5,
    })
    .collect();

    Ok(Json(members))
}

#[derive(serde::Deserialize)]
pub struct RemoveMemberPath {
    pub trip_id: String,
    pub user_id: String,
}

pub async fn remove_member(
    State(state): State<AppState>,
    access: TripAccess,
    Path(params): Path<RemoveMemberPath>,
) -> Result<StatusCode, AppError> {
    access.require_owner()?;

    if params.user_id == access.user.id {
        return Err(AppError::BadRequest(
            "Cannot remove yourself as owner".into(),
        ));
    }

    let result = sqlx::query("DELETE FROM trip_members WHERE trip_id = ?1 AND user_id = ?2")
        .bind(&access.trip_id)
        .bind(&params.user_id)
        .execute(&state.db.write)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}
