//! Wire-format and wallet-storage round-trip tests.
//!
//! Goal: pin down the JSON shapes of types that cross trust boundaries
//! (MintRequest / MintResponse / PublicKeyset / ReviewPayload) plus the
//! wallet-stored Credential. A change that breaks these tests is a wire
//! format change and requires a `chaumstar/0.x` version bump.

use chaumstar_core::{
    Credential, DisclosureMask, Issuer, KeysetId, MemoryRegistry, MintContext, MintRequest,
    MintResponse, ProductCategory, PublicKeyset, PurchaseTier, ReviewBody, ReviewPayload,
    mint_finish, mint_start, publish, verify,
};

const ISSUER_ID: &str = "bean-and-beam-coffee";
const MERCHANT_ID: &str = "main-store";
const REVIEW_TIMESTAMP: &str = "2026-05-17T13:00:00Z";

fn make_review_body() -> ReviewBody {
    ReviewBody {
        text: "美味しかった".into(),
        rating: 5,
        merchant_id: MERCHANT_ID.into(),
        issuer_id: ISSUER_ID.into(),
        timestamp: REVIEW_TIMESTAMP.into(),
    }
}

fn mint_ctx() -> MintContext {
    MintContext {
        merchant_id: MERCHANT_ID.into(),
        purchase_tier: PurchaseTier::Mid,
        product_category: ProductCategory::Drinks,
    }
}

struct Fixtures {
    issuer: Issuer,
    keyset: PublicKeyset,
    mint_request: MintRequest,
    mint_response: MintResponse,
    credential: Credential,
    payload: ReviewPayload,
}

fn fixtures() -> Fixtures {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();
    let (state, mint_request) = mint_start(&keyset, &mint_ctx()).unwrap();
    let mint_response = issuer.blind_sign(&mint_request).unwrap();
    let credential = mint_finish(state, mint_response.clone()).unwrap();
    let payload = publish(
        &credential,
        make_review_body(),
        DisclosureMask {
            disclose_tier: true,
            disclose_category: false,
        },
    )
    .unwrap();
    Fixtures {
        issuer,
        keyset,
        mint_request,
        mint_response,
        credential,
        payload,
    }
}

#[test]
fn public_keyset_json_round_trip() {
    let Fixtures { keyset, .. } = fixtures();
    let json = serde_json::to_string(&keyset).unwrap();
    let parsed: PublicKeyset = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.issuer_id, keyset.issuer_id);
    assert_eq!(parsed.merchant_id, keyset.merchant_id);
    assert_eq!(parsed.keyset_id, keyset.keyset_id);
    assert_eq!(parsed.public_key_bytes, keyset.public_key_bytes);
}

#[test]
fn keyset_id_serializes_as_16_char_hex_string() {
    let id = KeysetId([0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03, 0x04]);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"deadbeef01020304\"");
    let parsed: KeysetId = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, id);
}

#[test]
fn mint_request_json_round_trip() {
    let Fixtures { mint_request, .. } = fixtures();
    let json = serde_json::to_string(&mint_request).unwrap();
    let parsed: MintRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.issuer_id, mint_request.issuer_id);
    assert_eq!(parsed.merchant_id, mint_request.merchant_id);
    assert_eq!(parsed.keyset_id, mint_request.keyset_id);
    assert_eq!(parsed.commitment_bytes, mint_request.commitment_bytes);
}

#[test]
fn mint_response_json_round_trip_still_finishes() {
    let Fixtures {
        issuer,
        keyset,
        mint_response,
        ..
    } = fixtures();
    // Round-trip the response and rerun mint_finish against a fresh state.
    let json = serde_json::to_string(&mint_response).unwrap();
    let parsed: MintResponse = serde_json::from_str(&json).unwrap();

    // Need a matching state — re-run mint_start, but the resulting blind sig
    // is bound to that state's commitment, so we can't reuse `mint_response`
    // here. Instead just check structural equality.
    assert_eq!(
        parsed.blind_signature_bytes,
        mint_response.blind_signature_bytes
    );

    // And separately confirm the issuer still works after a response round-trip
    // by running a fresh mint end-to-end.
    let (state, req) = mint_start(&keyset, &mint_ctx()).unwrap();
    let resp = issuer.blind_sign(&req).unwrap();
    let resp_json = serde_json::to_string(&resp).unwrap();
    let resp_parsed: MintResponse = serde_json::from_str(&resp_json).unwrap();
    let _credential = mint_finish(state, resp_parsed).unwrap();
}

#[test]
fn credential_json_round_trip_preserves_secret_material() {
    let Fixtures { credential, .. } = fixtures();
    let json = serde_json::to_string(&credential).unwrap();
    let parsed: Credential = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.hpk, credential.hpk);
    assert_eq!(parsed.hsk, credential.hsk);
    assert_eq!(parsed.blind_factor, credential.blind_factor);
    assert_eq!(parsed.blind_signature, credential.blind_signature);
    assert_eq!(parsed.merchant_id, credential.merchant_id);
    assert_eq!(parsed.purchase_tier, credential.purchase_tier);
    assert_eq!(parsed.product_category, credential.product_category);
    assert_eq!(parsed.keyset.keyset_id, credential.keyset.keyset_id);
}

#[test]
fn credential_round_trip_can_still_publish_and_verify() {
    let Fixtures { credential, .. } = fixtures();
    let json = serde_json::to_string(&credential).unwrap();
    let parsed: Credential = serde_json::from_str(&json).unwrap();

    let payload = publish(&parsed, make_review_body(), DisclosureMask::default()).unwrap();
    verify(&payload, &parsed.keyset, &mut MemoryRegistry::default()).unwrap();
}

#[test]
fn review_payload_json_round_trip_still_verifies() {
    let Fixtures {
        keyset, payload, ..
    } = fixtures();

    let json = serde_json::to_string(&payload).unwrap();
    let parsed: ReviewPayload = serde_json::from_str(&json).unwrap();

    // Spot-check fields
    assert_eq!(parsed.review_body.text, payload.review_body.text);
    assert_eq!(parsed.credential_proof.hpk, payload.credential_proof.hpk);
    assert_eq!(
        parsed.credential_proof.bbs_proof,
        payload.credential_proof.bbs_proof
    );
    assert_eq!(parsed.sig, payload.sig);

    // The round-tripped payload must still verify cryptographically.
    verify(&parsed, &keyset, &mut MemoryRegistry::default()).unwrap();
}
