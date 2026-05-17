//! End-to-end scenario tests for chaumstar-core (v0.2 with selective disclosure).
//!
//! Acceptance criteria: a reviewer can mint a credential with attestable
//! attributes (purchase_tier, product_category), then publish a review choosing
//! which attributes to disclose. A reader can independently verify the proof.
//! Several attack classes — including reviewer lying about a disclosed value —
//! are detected.

use chaumstar_core::{
    DisclosureMask, Issuer, MemoryRegistry, MintContext, ProductCategory, PurchaseTier, ReviewBody,
    VerifyError, mint_finish, mint_start, publish, verify,
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

fn mint_ctx(tier: PurchaseTier, category: ProductCategory) -> MintContext {
    MintContext {
        merchant_id: MERCHANT_ID.into(),
        issued_at: ISSUED_AT.into(),
        purchase_tier: tier,
        product_category: category,
    }
}

#[test]
fn happy_path_mint_publish_verify_with_no_disclosure() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::Mid, ProductCategory::Drinks),
    )
    .expect("mint_start");
    let resp = issuer.blind_sign(&req).expect("blind_sign");
    let credential = mint_finish(state, resp).expect("mint_finish");

    let payload = publish(
        &credential,
        make_review_body("美味しかった", 5),
        DisclosureMask::default(),
    )
    .expect("publish");

    assert!(payload.credential_proof.purchase_tier.is_none());
    assert!(payload.credential_proof.product_category.is_none());

    verify(&payload, &keyset, &mut MemoryRegistry::default()).expect("verify");
}

#[test]
fn publish_disclosing_only_tier() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::High, ProductCategory::Food),
    )
    .expect("mint_start");
    let resp = issuer.blind_sign(&req).unwrap();
    let credential = mint_finish(state, resp).unwrap();

    let payload = publish(
        &credential,
        make_review_body("最高", 5),
        DisclosureMask {
            disclose_tier: true,
            disclose_category: false,
        },
    )
    .expect("publish");

    assert_eq!(
        payload.credential_proof.purchase_tier,
        Some(PurchaseTier::High)
    );
    assert!(payload.credential_proof.product_category.is_none());

    verify(&payload, &keyset, &mut MemoryRegistry::default()).expect("verify");
}

#[test]
fn publish_disclosing_both_attributes() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::Low, ProductCategory::Merch),
    )
    .expect("mint_start");
    let resp = issuer.blind_sign(&req).unwrap();
    let credential = mint_finish(state, resp).unwrap();

    let payload = publish(
        &credential,
        make_review_body("ok", 3),
        DisclosureMask {
            disclose_tier: true,
            disclose_category: true,
        },
    )
    .unwrap();

    assert_eq!(
        payload.credential_proof.purchase_tier,
        Some(PurchaseTier::Low)
    );
    assert_eq!(
        payload.credential_proof.product_category,
        Some(ProductCategory::Merch)
    );

    verify(&payload, &keyset, &mut MemoryRegistry::default()).expect("verify");
}

#[test]
fn double_review_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();
    let mut registry = MemoryRegistry::default();

    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::Mid, ProductCategory::Drinks),
    )
    .unwrap();
    let resp = issuer.blind_sign(&req).unwrap();
    let credential = mint_finish(state, resp).unwrap();

    let p1 = publish(
        &credential,
        make_review_body("ok", 4),
        DisclosureMask::default(),
    )
    .unwrap();
    verify(&p1, &keyset, &mut registry).expect("first verify");

    let p2 = publish(
        &credential,
        make_review_body("again", 3),
        DisclosureMask::default(),
    )
    .unwrap();
    let err = verify(&p2, &keyset, &mut registry).unwrap_err();
    assert!(
        matches!(err, VerifyError::AlreadyUsed),
        "expected AlreadyUsed, got {err:?}"
    );
}

#[test]
fn tampered_review_body_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::Mid, ProductCategory::Drinks),
    )
    .unwrap();
    let resp = issuer.blind_sign(&req).unwrap();
    let credential = mint_finish(state, resp).unwrap();

    let mut payload = publish(
        &credential,
        make_review_body("great", 5),
        DisclosureMask::default(),
    )
    .unwrap();
    payload.review_body.text = "terrible".into();

    let err = verify(&payload, &keyset, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(
            err,
            VerifyError::HolderSignatureInvalid | VerifyError::ProofInvalid
        ),
        "expected HolderSignatureInvalid or ProofInvalid, got {err:?}"
    );
}

#[test]
fn forged_credential_is_rejected() {
    let real = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let real_ks = real.public_keyset();

    let eve = Issuer::generate("eve-fake-coffee", MERCHANT_ID);
    let eve_ks = eve.public_keyset();

    let (state, req) = mint_start(
        &eve_ks,
        &mint_ctx(PurchaseTier::Mid, ProductCategory::Drinks),
    )
    .unwrap();
    let resp = eve.blind_sign(&req).unwrap();
    let cred = mint_finish(state, resp).unwrap();
    let payload = publish(
        &cred,
        make_review_body("fake", 1),
        DisclosureMask::default(),
    )
    .unwrap();

    let err = verify(&payload, &real_ks, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(err, VerifyError::ProofInvalid | VerifyError::KeysetMismatch),
        "expected ProofInvalid or KeysetMismatch, got {err:?}"
    );
}

#[test]
fn lying_about_disclosed_tier_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    // Mint with tier = Mid
    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::Mid, ProductCategory::Drinks),
    )
    .unwrap();
    let resp = issuer.blind_sign(&req).unwrap();
    let credential = mint_finish(state, resp).unwrap();

    // Publish disclosing tier truthfully
    let mut payload = publish(
        &credential,
        make_review_body("ok", 4),
        DisclosureMask {
            disclose_tier: true,
            disclose_category: false,
        },
    )
    .unwrap();
    assert_eq!(
        payload.credential_proof.purchase_tier,
        Some(PurchaseTier::Mid)
    );

    // Eve rewrites the disclosed tier from Mid → High
    payload.credential_proof.purchase_tier = Some(PurchaseTier::High);

    let err = verify(&payload, &keyset, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(
            err,
            VerifyError::ProofInvalid | VerifyError::HolderSignatureInvalid
        ),
        "expected ProofInvalid or HolderSignatureInvalid, got {err:?}"
    );
}

#[test]
fn pretending_undisclosed_attribute_was_disclosed_is_rejected() {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let keyset = issuer.public_keyset();

    let (state, req) = mint_start(
        &keyset,
        &mint_ctx(PurchaseTier::Mid, ProductCategory::Drinks),
    )
    .unwrap();
    let resp = issuer.blind_sign(&req).unwrap();
    let credential = mint_finish(state, resp).unwrap();

    // Publish without disclosing anything
    let mut payload = publish(
        &credential,
        make_review_body("ok", 4),
        DisclosureMask::default(),
    )
    .unwrap();
    assert!(payload.credential_proof.purchase_tier.is_none());

    // Eve inserts a fabricated tier into the payload after the fact
    payload.credential_proof.purchase_tier = Some(PurchaseTier::High);

    let err = verify(&payload, &keyset, &mut MemoryRegistry::default()).unwrap_err();
    assert!(
        matches!(
            err,
            VerifyError::ProofInvalid | VerifyError::HolderSignatureInvalid
        ),
        "expected ProofInvalid or HolderSignatureInvalid, got {err:?}"
    );
}

#[test]
fn purchase_tier_from_yen_buckets_correctly() {
    assert_eq!(PurchaseTier::from_yen(0), PurchaseTier::Low);
    assert_eq!(PurchaseTier::from_yen(999), PurchaseTier::Low);
    assert_eq!(PurchaseTier::from_yen(1_000), PurchaseTier::Mid);
    assert_eq!(PurchaseTier::from_yen(4_999), PurchaseTier::Mid);
    assert_eq!(PurchaseTier::from_yen(5_000), PurchaseTier::High);
    assert_eq!(PurchaseTier::from_yen(99_999), PurchaseTier::High);
}
