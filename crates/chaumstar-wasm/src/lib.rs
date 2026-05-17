//! chaumstar-wasm
//!
//! Browser bindings for the chaumstar wallet + verifier. Build with:
//! `wasm-pack build crates/chaumstar-wasm --target web`.
//!
//! All wallet-side cryptography (Ed25519 keygen, BBS+ blind issuance unblinding,
//! BBS+ proof generation, Ed25519 signing) runs inside this WASM module, so the
//! reviewer's secret material never leaves the browser. The reader's BBS+
//! presentation-proof verification also runs here so the reader does not have
//! to trust the registry's "verified" claim.

use chaumstar_core::{
    Credential, MintRequest, MintResponse, MintState, PublicKeyset, ReviewBody, ReviewPayload,
    mint_finish as core_mint_finish, mint_start as core_mint_start, publish as core_publish,
    verify_proof_only,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = protocolVersion)]
pub fn protocol_version() -> String {
    chaumstar_core::PROTOCOL_VERSION.to_string()
}

#[derive(Serialize)]
struct MintStartResult {
    state: MintState,
    request: MintRequest,
}

/// Start a mint: generate a fresh Ed25519 keypair, build the BBS+ commitment,
/// and return both the wallet-side state and the wire-format request.
#[wasm_bindgen(js_name = mintStart)]
pub fn mint_start(keyset: JsValue, merchant_id: &str, issued_at: &str) -> Result<JsValue, JsError> {
    let keyset: PublicKeyset = serde_wasm_bindgen::from_value(keyset)
        .map_err(|e| JsError::new(&format!("keyset: {e}")))?;
    let (state, request) = core_mint_start(&keyset, merchant_id, issued_at)
        .map_err(|e| JsError::new(&format!("mint_start: {e}")))?;
    Ok(serde_wasm_bindgen::to_value(&MintStartResult {
        state,
        request,
    })?)
}

/// Finish a mint: verify the issuer's blind signature locally and produce a
/// [`Credential`] ready for publication.
#[wasm_bindgen(js_name = mintFinish)]
pub fn mint_finish(state: JsValue, response: JsValue) -> Result<JsValue, JsError> {
    let state: MintState =
        serde_wasm_bindgen::from_value(state).map_err(|e| JsError::new(&format!("state: {e}")))?;
    let response: MintResponse = serde_wasm_bindgen::from_value(response)
        .map_err(|e| JsError::new(&format!("response: {e}")))?;
    let credential = core_mint_finish(state, response)
        .map_err(|e| JsError::new(&format!("mint_finish: {e}")))?;
    Ok(serde_wasm_bindgen::to_value(&credential)?)
}

/// Publish a review: generate a BBS+ presentation proof + Ed25519 holder
/// signature bound to the canonical review body.
#[wasm_bindgen(js_name = publishReview)]
pub fn publish_review(credential: JsValue, body: JsValue) -> Result<JsValue, JsError> {
    let credential: Credential = serde_wasm_bindgen::from_value(credential)
        .map_err(|e| JsError::new(&format!("credential: {e}")))?;
    let body: ReviewBody =
        serde_wasm_bindgen::from_value(body).map_err(|e| JsError::new(&format!("body: {e}")))?;
    let payload =
        core_publish(&credential, body).map_err(|e| JsError::new(&format!("publish: {e}")))?;
    Ok(serde_wasm_bindgen::to_value(&payload)?)
}

/// Verify the cryptographic parts of a payload against the issuer's public
/// keyset. Does NOT check the nullifier registry — the reader is expected to
/// either trust the registry server's freshness claim or run its own.
#[wasm_bindgen(js_name = verifyProof)]
pub fn verify_proof(payload: JsValue, keyset: JsValue) -> Result<(), JsError> {
    let payload: ReviewPayload = serde_wasm_bindgen::from_value(payload)
        .map_err(|e| JsError::new(&format!("payload: {e}")))?;
    let keyset: PublicKeyset = serde_wasm_bindgen::from_value(keyset)
        .map_err(|e| JsError::new(&format!("keyset: {e}")))?;
    verify_proof_only(&payload, &keyset).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(())
}
