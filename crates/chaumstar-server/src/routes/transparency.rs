use axum::{Json, extract::State};
use serde::Serialize;

use crate::{error::ApiError, state::AppState};

#[derive(Serialize)]
pub struct RegistryKeyResponse {
    pub public_key: String,
}

pub async fn registry_key(State(state): State<AppState>) -> Json<RegistryKeyResponse> {
    let pk = state.reviews().registry_public_key();
    Json(RegistryKeyResponse {
        public_key: hex::encode(pk),
    })
}

pub async fn sth(State(state): State<AppState>) -> Result<Json<chaumstar_core::Sth>, ApiError> {
    state
        .reviews()
        .current_sth()
        .map(Json)
        .map_err(|e| ApiError::Internal(format!("sth: {e:?}")))
}
