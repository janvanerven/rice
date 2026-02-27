use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trip {
    pub id: String,
    pub name: String,
    pub destination: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub cover_image_path: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TripMember {
    pub trip_id: String,
    pub user_id: String,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripWithRole {
    #[serde(flatten)]
    pub trip: Trip,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invite {
    pub id: String,
    pub trip_id: String,
    pub email: String,
    pub token_hash: String,
    pub role: String,
    pub expires_at: String,
    pub claimed_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub refresh_token_hash: String,
    pub expires_at: String,
    pub created_at: String,
}

// API request/response types
#[derive(Debug, Deserialize)]
pub struct CreateTripRequest {
    pub name: String,
    pub destination: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTripRequest {
    pub name: Option<String>,
    pub destination: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Accommodation {
    pub id: String,
    pub trip_id: String,
    pub name: String,
    pub address: Option<String>,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub notes: Option<String>,
    pub cover_image_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImageAttribution {
    pub entity_type: String,
    pub entity_id: String,
    pub author_name: String,
    pub author_url: String,
    pub source_url: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    pub author_name: String,
    pub author_url: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCoverResponse {
    pub path: String,
    pub attribution: Attribution,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccommodationRequest {
    pub name: String,
    pub address: Option<String>,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccommodationRequest {
    pub name: Option<String>,
    pub address: Option<String>,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub notes: Option<String>,
}
