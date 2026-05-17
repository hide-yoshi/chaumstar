use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chaumstar_core::{MintRequest, MintResponse};

use crate::{error::ApiError, state::AppState};

pub async fn create(
    State(state): State<AppState>,
    Json(request): Json<MintRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let issuer = state
        .get_issuer(&request.keyset_id)
        .ok_or_else(|| ApiError::BadRequest("unknown keyset_id".into()))?;

    let response: MintResponse = issuer
        .blind_sign(&request)
        .map_err(|e| ApiError::BadRequest(format!("blind_sign failed: {e}")))?;

    Ok((StatusCode::CREATED, Json(response)))
}
