use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chaumstar_core::{InclusionProof, ReviewPayload, Sth};
use serde::Serialize;

use crate::{
    error::ApiError,
    state::{AppState, InsertError},
};

#[derive(Serialize)]
pub struct ReviewWithProof {
    pub payload: ReviewPayload,
    pub inclusion_proof: InclusionProof,
}

#[derive(Serialize)]
pub struct ReviewWithProofAndSth {
    pub payload: ReviewPayload,
    pub inclusion_proof: InclusionProof,
    pub sth: Sth,
}

#[derive(Serialize)]
pub struct ReviewListResponse {
    pub reviews: Vec<ReviewWithProof>,
    pub sth: Sth,
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<ReviewPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let issuer = state
        .get_issuer(&payload.credential_proof.keyset_id)
        .ok_or_else(|| ApiError::BadRequest("unknown keyset_id".into()))?;
    let keyset = issuer.public_keyset();

    match state.reviews().check_and_insert(payload, &keyset) {
        Ok((payload, inclusion_proof, sth)) => Ok((
            StatusCode::CREATED,
            Json(ReviewWithProofAndSth {
                payload,
                inclusion_proof,
                sth,
            }),
        )),
        Err(InsertError::AlreadyUsed) => Err(ApiError::Conflict(
            "credential nullifier already used".into(),
        )),
        Err(InsertError::Verify(e)) => {
            Err(ApiError::BadRequest(format!("verification failed: {e}")))
        }
        Err(InsertError::Internal(e)) => Err(ApiError::Internal(e)),
    }
}

pub async fn list(State(state): State<AppState>) -> Result<Json<ReviewListResponse>, ApiError> {
    let (rows, sth) = state
        .reviews()
        .list_with_proofs()
        .map_err(|e| ApiError::Internal(format!("{e:?}")))?;
    let reviews = rows
        .into_iter()
        .map(|(payload, inclusion_proof)| ReviewWithProof {
            payload,
            inclusion_proof,
        })
        .collect();
    Ok(Json(ReviewListResponse { reviews, sth }))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(hpk_hex): Path<String>,
) -> Result<Json<ReviewWithProofAndSth>, ApiError> {
    let bytes = hex::decode(&hpk_hex)
        .map_err(|_| ApiError::BadRequest("hpk must be 64-char hex".into()))?;
    let hpk: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ApiError::BadRequest("hpk must decode to 32 bytes".into()))?;

    let opt = state
        .reviews()
        .get_with_proof(&hpk)
        .map_err(|e| ApiError::Internal(format!("{e:?}")))?;
    let (payload, inclusion_proof, sth) = opt.ok_or(ApiError::NotFound)?;
    Ok(Json(ReviewWithProofAndSth {
        payload,
        inclusion_proof,
        sth,
    }))
}
