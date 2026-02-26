use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use cookie::Cookie;

use crate::{auth::jwt, errors::AppError, models::User, AppState};

/// Extracts the authenticated user from the JWT cookie.
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookie_header = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

        let mut access_token = None;
        for cookie_part in cookie_header.split(';') {
            if let Ok(c) = Cookie::parse(cookie_part.trim()) {
                if c.name() == "rice_access" {
                    access_token = Some(c.value().to_string());
                }
            }
        }

        let token =
            access_token.ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))?;

        let claims = jwt::verify_token(&token, &state.config.jwt_secret)
            .map_err(|_| AppError::Unauthorized("Invalid or expired token".into()))?;

        // Verify session still exists (revocation check)
        let session_exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sessions WHERE id = ?1 AND expires_at > datetime('now')",
        )
        .bind(&claims.jti)
        .fetch_one(&state.db.read)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if !session_exists {
            return Err(AppError::Unauthorized(
                "Session revoked or expired".into(),
            ));
        }

        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
            .bind(&claims.sub)
            .fetch_optional(&state.db.read)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("User not found".into()))?;

        Ok(AuthUser(user))
    }
}

/// Extracts authenticated user + verifies trip membership.
pub struct TripAccess {
    pub user: User,
    pub trip_id: String,
    pub role: String,
}

impl TripAccess {
    pub fn require_editor(&self) -> Result<(), AppError> {
        match self.role.as_str() {
            "owner" | "editor" => Ok(()),
            _ => Err(AppError::Forbidden("Editor access required".into())),
        }
    }

    pub fn require_owner(&self) -> Result<(), AppError> {
        if self.role == "owner" {
            Ok(())
        } else {
            Err(AppError::Forbidden("Owner access required".into()))
        }
    }
}

impl FromRequestParts<AppState> for TripAccess {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;

        let Path(params): Path<std::collections::HashMap<String, String>> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::BadRequest("Missing trip_id in path".into()))?;

        let trip_id = params
            .get("trip_id")
            .ok_or_else(|| AppError::BadRequest("Missing trip_id parameter".into()))?
            .clone();

        let member: Option<crate::models::TripMember> = sqlx::query_as(
            "SELECT * FROM trip_members WHERE trip_id = ?1 AND user_id = ?2",
        )
        .bind(&trip_id)
        .bind(&user.id)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let member = member.ok_or_else(|| AppError::NotFound("Trip not found".into()))?;

        Ok(TripAccess {
            user,
            trip_id,
            role: member.role,
        })
    }
}
