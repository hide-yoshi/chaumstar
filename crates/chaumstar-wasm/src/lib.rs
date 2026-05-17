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
    Credential, DisclosureMask, InclusionProof, MintContext, MintRequest, MintResponse, MintState,
    PublicKeyset, ReviewBody, ReviewPayload, Sth, leaf_hash, mint_finish as core_mint_finish,
    mint_start as core_mint_start, publish as core_publish, verify_proof_only,
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
///
/// `ctx` must serialize to a `MintContext` (`merchant_id`, `purchase_tier`,
/// `product_category`).
#[wasm_bindgen(js_name = mintStart)]
pub fn mint_start(keyset: JsValue, ctx: JsValue) -> Result<JsValue, JsError> {
    let keyset: PublicKeyset = serde_wasm_bindgen::from_value(keyset)
        .map_err(|e| JsError::new(&format!("keyset: {e}")))?;
    let ctx: MintContext =
        serde_wasm_bindgen::from_value(ctx).map_err(|e| JsError::new(&format!("ctx: {e}")))?;
    let (state, request) =
        core_mint_start(&keyset, &ctx).map_err(|e| JsError::new(&format!("mint_start: {e}")))?;
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
/// signature bound to the canonical review body. `mask` selects which of
/// `purchase_tier` / `product_category` are disclosed in the published
/// payload. Passing `undefined` / `null` defaults to disclosing nothing.
#[wasm_bindgen(js_name = publishReview)]
pub fn publish_review(
    credential: JsValue,
    body: JsValue,
    mask: JsValue,
) -> Result<JsValue, JsError> {
    let credential: Credential = serde_wasm_bindgen::from_value(credential)
        .map_err(|e| JsError::new(&format!("credential: {e}")))?;
    let body: ReviewBody =
        serde_wasm_bindgen::from_value(body).map_err(|e| JsError::new(&format!("body: {e}")))?;
    let mask: DisclosureMask = if mask.is_undefined() || mask.is_null() {
        DisclosureMask::default()
    } else {
        serde_wasm_bindgen::from_value(mask).map_err(|e| JsError::new(&format!("mask: {e}")))?
    };
    let payload = core_publish(&credential, body, mask)
        .map_err(|e| JsError::new(&format!("publish: {e}")))?;
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

/// Verify a Signed Tree Head against the Registry's Ed25519 public key.
/// `registry_pubkey_hex` is the 32-byte ed25519 pubkey as 64-char hex.
#[wasm_bindgen(js_name = verifySth)]
pub fn verify_sth(sth: JsValue, registry_pubkey_hex: &str) -> Result<(), JsError> {
    let sth: Sth =
        serde_wasm_bindgen::from_value(sth).map_err(|e| JsError::new(&format!("sth: {e}")))?;
    let bytes = hex::decode(registry_pubkey_hex)
        .map_err(|e| JsError::new(&format!("registry_pubkey hex: {e}")))?;
    let pk: [u8; 32] = bytes
        .try_into()
        .map_err(|_| JsError::new("registry_pubkey must be 32 bytes"))?;
    sth.verify(&pk).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(())
}

/// Verify a payload's Merkle inclusion proof against a known STH root.
/// Recomputes the leaf hash from the payload, walks the proof path, and
/// checks the resulting root equals `sth.root_hash`.
#[wasm_bindgen(js_name = verifyInclusion)]
pub fn verify_inclusion(payload: JsValue, proof: JsValue, sth: JsValue) -> Result<(), JsError> {
    let payload: ReviewPayload = serde_wasm_bindgen::from_value(payload)
        .map_err(|e| JsError::new(&format!("payload: {e}")))?;
    let proof: InclusionProof =
        serde_wasm_bindgen::from_value(proof).map_err(|e| JsError::new(&format!("proof: {e}")))?;
    let sth: Sth =
        serde_wasm_bindgen::from_value(sth).map_err(|e| JsError::new(&format!("sth: {e}")))?;
    let leaf = leaf_hash(&payload).map_err(|e| JsError::new(&format!("leaf hash: {e}")))?;
    proof
        .verify(&leaf, &sth.root_hash)
        .map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(())
}
