//! End-to-end scenario tests for chaumstar-core.
//!
//! These tests drive the public API design from the outside in.
//! Acceptance criteria: a reviewer can mint a credential, publish a review,
//! and a reader can verify it — while four attack classes are detected.

use chaumstar_core::{
    Issuer, MemoryRegistry, ReviewBody, VerifyError, mint_finish, mint_start, publish, verify,
};

const ISSUER_ID: &str = "bean-and-beam-coffee";
const MERCHANT_ID: &str = "main-store";
const ISSUED_AT: &str = "2026-05-17T10:00:00Z";
const REVIEW_TIMESTAMP: &str = "2026-05-17T13:00:00Z";

fn make_review_body(text: &str, rating: u8) -> ReviewBody {
    ReviewBody {
        text: text.to_string(),
        rating,
        merchant_id: MERCHANT_ID.to_string(),
        issuer_id: ISSUER_ID.to_string(),
        issued_at: ISSUED_AT.to_string(),
        timestamp: REVIEW_TIMESTAMP.to_string(),
    }
}

#[test]
fn happy_path_mint_publish_verify() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    // Mint
    let (mint_state, mint_request) =
        mint_start(&keyset, MERCHANT_ID, ISSUED_AT).expect("mint_start");
    let mint_response = issuer.blind_sign(&mint_request).expect("issuer.blind_sign");
    let credential = mint_finish(mint_state, mint_response).expect("mint_finish");

    // Publish
    let body = make_review_body("美味しかった", 5);
    let payload = publish(&credential, body).expect("publish");

    // Verify
    let mut registry = MemoryRegistry::default();
    verify(&payload, &keyset, &mut registry).expect("verify");
}

#[test]
fn double_review_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();
    let mut registry = MemoryRegistry::default();

    let (mint_state, mint_request) = mint_start(&keyset, MERCHANT_ID, ISSUED_AT).unwrap();
    let mint_response = issuer.blind_sign(&mint_request).unwrap();
    let credential = mint_finish(mint_state, mint_response).unwrap();

    // First publish — succeeds
    let payload_1 = publish(&credential, make_review_body("ok", 4)).unwrap();
    verify(&payload_1, &keyset, &mut registry).expect("first verify");

    // Second publish with same credential — must be rejected
    let payload_2 = publish(&credential, make_review_body("again", 3)).unwrap();
    let err = verify(&payload_2, &keyset, &mut registry).unwrap_err();
    assert!(
        matches!(err, VerifyError::AlreadyUsed),
        "expected AlreadyUsed, got {err:?}",
    );
}

#[test]
fn tampered_review_body_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (mint_state, mint_request) = mint_start(&keyset, MERCHANT_ID, ISSUED_AT).unwrap();
    let mint_response = issuer.blind_sign(&mint_request).unwrap();
    let credential = mint_finish(mint_state, mint_response).unwrap();

    let mut payload = publish(&credential, make_review_body("great", 5)).unwrap();
    // Tamper after signing
    payload.review_body.text = "terrible".to_string();

    let err = verify(&payload, &keyset, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(
            err,
            VerifyError::HolderSignatureInvalid | VerifyError::ProofInvalid
        ),
        "expected HolderSignatureInvalid or ProofInvalid, got {err:?}",
    );
}

#[test]
fn forged_credential_is_rejected() {
    // Eve makes a credential under her own issuer key but tries to publish
    // a review under the *real* issuer's keyset.
    let real_issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let real_keyset = real_issuer.public_keyset();

    let eve_issuer = Issuer::generate("eve-fake-coffee", MERCHANT_ID);
    let eve_keyset = eve_issuer.public_keyset();

    let (mint_state, mint_request) = mint_start(&eve_keyset, MERCHANT_ID, ISSUED_AT).unwrap();
    let mint_response = eve_issuer.blind_sign(&mint_request).unwrap();
    let eve_credential = mint_finish(mint_state, mint_response).unwrap();

    let payload = publish(&eve_credential, make_review_body("fake", 1)).unwrap();

    // Verify against the real issuer's keyset
    let err = verify(&payload, &real_keyset, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(err, VerifyError::ProofInvalid | VerifyError::KeysetMismatch),
        "expected ProofInvalid or KeysetMismatch, got {err:?}",
    );
}

#[test]
fn wrong_merchant_id_in_payload_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (mint_state, mint_request) = mint_start(&keyset, MERCHANT_ID, ISSUED_AT).unwrap();
    let mint_response = issuer.blind_sign(&mint_request).unwrap();
    let credential = mint_finish(mint_state, mint_response).unwrap();

    let mut payload = publish(&credential, make_review_body("ok", 5)).unwrap();
    // Pretend the review was for a different merchant after the fact
    payload.review_body.merchant_id = "different-store".to_string();

    let err = verify(&payload, &keyset, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(
            err,
            VerifyError::HolderSignatureInvalid
                | VerifyError::ProofInvalid
                | VerifyError::KeysetMismatch
        ),
        "expected verification failure, got {err:?}",
    );
}
