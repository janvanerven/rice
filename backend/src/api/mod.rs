pub mod trips;

use axum::{routing::get, Router};
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
}
