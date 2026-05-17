use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chaumstar_core::ReviewPayload;

use crate::{
    error::ApiError,
    state::{AppState, InsertError},
};

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<ReviewPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let issuer = state
        .get_issuer(&payload.credential_proof.keyset_id)
        .ok_or_else(|| ApiError::BadRequest("unknown keyset_id".into()))?;
    let keyset = issuer.public_keyset();

    match state.reviews().check_and_insert(payload.clone(), &keyset) {
        Ok(()) => Ok((StatusCode::CREATED, Json(payload))),
        Err(InsertError::AlreadyUsed) => Err(ApiError::Conflict(
            "credential nullifier already used".into(),
        )),
        Err(InsertError::Verify(e)) => {
            Err(ApiError::BadRequest(format!("verification failed: {e}")))
        }
    }
}

pub async fn list(State(state): State<AppState>) -> Json<Vec<ReviewPayload>> {
    Json(state.reviews().list())
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(hpk_hex): Path<String>,
) -> Result<Json<ReviewPayload>, ApiError> {
    let bytes = hex::decode(&hpk_hex)
        .map_err(|_| ApiError::BadRequest("hpk must be 64-char hex".into()))?;
    let hpk: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ApiError::BadRequest("hpk must decode to 32 bytes".into()))?;

    state
        .reviews()
        .get(&hpk)
        .map(Json)
        .ok_or(ApiError::NotFound)
}
