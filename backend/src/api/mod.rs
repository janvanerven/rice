pub mod accommodations;
pub mod auto_cover;
pub mod invites;
pub mod members;
pub mod trips;
pub mod uploads;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/trips", get(trips::list_trips).post(trips::create_trip))
        .route(
            "/api/trips/{trip_id}",
            get(trips::get_trip)
                .put(trips::update_trip)
                .delete(trips::delete_trip),
        )
        .route("/api/trips/{trip_id}/members", get(members::list_members))
        .route(
            "/api/trips/{trip_id}/members/{user_id}",
            delete(members::remove_member),
        )
        .route(
            "/api/trips/{trip_id}/invites",
            post(invites::create_invite),
        )
        .route(
            "/api/trips/{trip_id}/cover",
            post(uploads::upload_cover),
        )
        .route(
            "/api/trips/{trip_id}/auto-cover",
            post(auto_cover::auto_cover_trip),
        )
        .route(
            "/api/trips/{trip_id}/accommodations",
            get(accommodations::list_accommodations)
                .post(accommodations::create_accommodation),
        )
        .route(
            "/api/trips/{trip_id}/accommodations/{accommodation_id}",
            put(accommodations::update_accommodation)
                .delete(accommodations::delete_accommodation),
        )
        .route(
            "/api/trips/{trip_id}/accommodations/{accommodation_id}/cover",
            post(accommodations::upload_accommodation_cover),
        )
        .route(
            "/api/trips/{trip_id}/accommodations/{accommodation_id}/auto-cover",
            post(auto_cover::auto_cover_accommodation),
        )
}
