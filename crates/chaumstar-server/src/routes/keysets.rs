use axum::{
    Json,
    extract::{Path, State},
};
use chaumstar_core::{KeysetId, PublicKeyset};

use crate::{error::ApiError, state::AppState};

pub async fn list(State(state): State<AppState>) -> Json<Vec<PublicKeyset>> {
    Json(state.list_keysets())
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(kid_hex): Path<String>,
) -> Result<Json<PublicKeyset>, ApiError> {
    let bytes = hex::decode(&kid_hex)
        .map_err(|_| ApiError::BadRequest("kid must be 16-char hex".into()))?;
    let arr: [u8; 8] = bytes
        .try_into()
        .map_err(|_| ApiError::BadRequest("kid must decode to 8 bytes".into()))?;
    let kid = KeysetId(arr);

    let issuer = state.get_issuer(&kid).ok_or(ApiError::NotFound)?;
    Ok(Json(issuer.public_keyset()))
}
