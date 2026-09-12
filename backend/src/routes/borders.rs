use crate::state::AppState;
use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub async fn handler(State(state): State<Arc<AppState>>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/geo+json")],
        state.borders.geojson_str.clone(),
    )
        .into_response()
}
