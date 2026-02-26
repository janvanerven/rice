pub mod invites;
pub mod members;
pub mod trips;

use axum::{
    routing::{delete, get, post},
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
}
