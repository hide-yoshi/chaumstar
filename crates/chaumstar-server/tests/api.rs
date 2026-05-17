//! HTTP-level acceptance tests for chaumstar-server.
//!
//! These drive the public API design from the outside in.
//! Uses `tower::ServiceExt::oneshot` to exercise the axum router directly,
//! no network involved.

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chaumstar_core::{
    DisclosureMask, InclusionProof, Issuer, MintContext, MintResponse, ProductCategory,
    PublicKeyset, PurchaseTier, ReviewBody, ReviewPayload, Sth, leaf_hash, mint_finish, mint_start,
    publish,
};
use chaumstar_server::{AppState, build_router};
use http_body_util::BodyExt;
use serde::Deserialize;
use tower::ServiceExt;

const ISSUER_ID: &str = "bean-and-beam-coffee";
const MERCHANT_ID: &str = "main-store";
const REVIEW_TIMESTAMP: &str = "2026-05-17T13:00:00Z";

fn make_review_body(text: &str, rating: u8) -> ReviewBody {
    ReviewBody {
        text: text.into(),
        rating,
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

fn fresh_state() -> AppState {
    let issuer = Issuer::generate(ISSUER_ID, MERCHANT_ID);
    let state = AppState::new();
    state.register_issuer(issuer);
    state
}

async fn read_json<T: serde::de::DeserializeOwned>(body: Body) -> T {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("decoding {}: {e}", String::from_utf8_lossy(&bytes)))
}

fn post_json<T: serde::Serialize>(uri: &str, value: &T) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(value).unwrap()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[derive(Deserialize)]
struct ReviewWithProof {
    payload: ReviewPayload,
    inclusion_proof: InclusionProof,
}

#[derive(Deserialize)]
struct ReviewWithProofAndSth {
    payload: ReviewPayload,
    inclusion_proof: InclusionProof,
    sth: Sth,
}

#[derive(Deserialize)]
struct ReviewListResponse {
    reviews: Vec<ReviewWithProof>,
    sth: Sth,
}

#[derive(Deserialize)]
struct RegistryKeyResponse {
    public_key: String,
}

fn parse_registry_pubkey(s: &str) -> [u8; 32] {
    let v = hex::decode(s).expect("hex");
    v.try_into().expect("32 bytes")
}

#[tokio::test]
async fn health_endpoint_returns_200() {
    let app = build_router(fresh_state());
    let response = app.oneshot(get("/api/v1/health")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn list_keysets_returns_registered_issuer() {
    let app = build_router(fresh_state());
    let response = app.oneshot(get("/api/v1/keysets")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let keysets: Vec<PublicKeyset> = read_json(response.into_body()).await;
    assert_eq!(keysets.len(), 1);
    assert_eq!(keysets[0].issuer_id, ISSUER_ID);
    assert_eq!(keysets[0].merchant_id, MERCHANT_ID);
}

#[tokio::test]
async fn get_keyset_by_kid_works_and_404s_for_unknown() {
    let app = build_router(fresh_state());

    // discover the kid
    let listed: Vec<PublicKeyset> = read_json(
        app.clone()
            .oneshot(get("/api/v1/keysets"))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let kid_hex = listed[0].keyset_id.to_hex();

    let response = app
        .clone()
        .oneshot(get(&format!("/api/v1/keysets/{kid_hex}")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(get("/api/v1/keysets/0000000000000000"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn mint_endpoint_returns_201_and_response_finishes_locally() {
    let state = fresh_state();
    let keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    let (mint_state, mint_req) = mint_start(&keyset, &mint_ctx()).unwrap();
    let response = app
        .oneshot(post_json("/api/v1/mints", &mint_req))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let mint_resp: MintResponse = read_json(response.into_body()).await;
    let _credential = mint_finish(mint_state, mint_resp).expect("mint_finish");
}

#[tokio::test]
async fn mint_endpoint_rejects_unknown_keyset_400() {
    let state = fresh_state();
    let real_keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    // Forge a request that targets a keyset the server does not know.
    let (_state, mut req) = mint_start(&real_keyset, &mint_ctx()).unwrap();
    req.keyset_id = chaumstar_core::KeysetId([1, 2, 3, 4, 5, 6, 7, 8]);

    let response = app.oneshot(post_json("/api/v1/mints", &req)).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn review_post_then_list_round_trips() {
    let state = fresh_state();
    let keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    // Mint a credential via the server.
    let (mint_state, mint_req) = mint_start(&keyset, &mint_ctx()).unwrap();
    let mint_response: MintResponse = read_json(
        app.clone()
            .oneshot(post_json("/api/v1/mints", &mint_req))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let credential = mint_finish(mint_state, mint_response).unwrap();
    let payload = publish(
        &credential,
        make_review_body("美味しかった", 5),
        DisclosureMask::default(),
    )
    .unwrap();

    // POST the review
    let response = app
        .clone()
        .oneshot(post_json("/api/v1/reviews", &payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ReviewWithProofAndSth = read_json(response.into_body()).await;
    assert_eq!(created.payload.review_body.text, "美味しかった");
    assert_eq!(created.sth.tree_size, 1);
    assert_eq!(created.inclusion_proof.tree_size, 1);

    // GET list
    let response = app.clone().oneshot(get("/api/v1/reviews")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let list: ReviewListResponse = read_json(response.into_body()).await;
    assert_eq!(list.reviews.len(), 1);
    assert_eq!(list.reviews[0].payload.review_body.text, "美味しかった");
    assert_eq!(list.sth.tree_size, 1);

    // GET by hpk
    let hpk_hex = hex::encode(payload.credential_proof.hpk);
    let response = app
        .oneshot(get(&format!("/api/v1/reviews/{hpk_hex}")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: ReviewWithProofAndSth = read_json(response.into_body()).await;
    assert_eq!(
        fetched.payload.credential_proof.hpk,
        payload.credential_proof.hpk
    );
}

#[tokio::test]
async fn review_post_double_spend_returns_409() {
    let state = fresh_state();
    let keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    let (mint_state, mint_req) = mint_start(&keyset, &mint_ctx()).unwrap();
    let mint_response: MintResponse = read_json(
        app.clone()
            .oneshot(post_json("/api/v1/mints", &mint_req))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let credential = mint_finish(mint_state, mint_response).unwrap();

    // First publish
    let payload_1 = publish(
        &credential,
        make_review_body("ok", 4),
        DisclosureMask::default(),
    )
    .unwrap();
    let r1 = app
        .clone()
        .oneshot(post_json("/api/v1/reviews", &payload_1))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::CREATED);

    // Second publish with same credential
    let payload_2 = publish(
        &credential,
        make_review_body("again", 3),
        DisclosureMask::default(),
    )
    .unwrap();
    let r2 = app
        .oneshot(post_json("/api/v1/reviews", &payload_2))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn review_post_tampered_body_returns_400() {
    let state = fresh_state();
    let keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    let (mint_state, mint_req) = mint_start(&keyset, &mint_ctx()).unwrap();
    let mint_response: MintResponse = read_json(
        app.clone()
            .oneshot(post_json("/api/v1/mints", &mint_req))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let credential = mint_finish(mint_state, mint_response).unwrap();

    let mut payload = publish(
        &credential,
        make_review_body("great", 5),
        DisclosureMask::default(),
    )
    .unwrap();
    payload.review_body.text = "tampered".into();

    let response = app
        .oneshot(post_json("/api/v1/reviews", &payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_review_unknown_hpk_returns_404() {
    let app = build_router(fresh_state());
    let response = app
        .oneshot(get(
            "/api/v1/reviews/00000000000000000000000000000000000000000000000000000000000000ff",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn registry_key_returns_32_byte_pubkey() {
    let app = build_router(fresh_state());
    let response = app.oneshot(get("/api/v1/registry-key")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: RegistryKeyResponse = read_json(response.into_body()).await;
    assert_eq!(parse_registry_pubkey(&body.public_key).len(), 32);
}

#[tokio::test]
async fn empty_log_returns_signed_zero_size_sth() {
    let app = build_router(fresh_state());
    let pk: RegistryKeyResponse = read_json(
        app.clone()
            .oneshot(get("/api/v1/registry-key"))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let pubkey = parse_registry_pubkey(&pk.public_key);

    let sth: Sth = read_json(app.oneshot(get("/api/v1/sth")).await.unwrap().into_body()).await;
    assert_eq!(sth.tree_size, 0);
    sth.verify(&pubkey).expect("empty sth verifies");
}

#[tokio::test]
async fn inclusion_proof_verifies_against_registry_sth() {
    let state = fresh_state();
    let keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    // Get registry pubkey
    let pk_body: RegistryKeyResponse = read_json(
        app.clone()
            .oneshot(get("/api/v1/registry-key"))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let pubkey = parse_registry_pubkey(&pk_body.public_key);

    // Publish two reviews
    let mut hpks: Vec<[u8; 32]> = vec![];
    for label in ["first", "second"] {
        let (mint_state, mint_req) = mint_start(&keyset, &mint_ctx()).unwrap();
        let mint_response: MintResponse = read_json(
            app.clone()
                .oneshot(post_json("/api/v1/mints", &mint_req))
                .await
                .unwrap()
                .into_body(),
        )
        .await;
        let credential = mint_finish(mint_state, mint_response).unwrap();
        let payload = publish(
            &credential,
            make_review_body(label, 4),
            DisclosureMask::default(),
        )
        .unwrap();
        let resp = app
            .clone()
            .oneshot(post_json("/api/v1/reviews", &payload))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: ReviewWithProofAndSth = read_json(resp.into_body()).await;
        hpks.push(body.payload.credential_proof.hpk);
    }

    // List + verify each inclusion proof against the listed sth
    let list: ReviewListResponse = read_json(
        app.clone()
            .oneshot(get("/api/v1/reviews"))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    assert_eq!(list.sth.tree_size, 2);
    list.sth.verify(&pubkey).expect("list sth signature ok");
    for row in &list.reviews {
        let leaf = leaf_hash(&row.payload).expect("leaf hash");
        row.inclusion_proof
            .verify(&leaf, &list.sth.root_hash)
            .expect("each inclusion proof verifies against current root");
    }
}

#[tokio::test]
async fn tampered_payload_breaks_inclusion_proof_against_stored_root() {
    let state = fresh_state();
    let keyset = state.first_public_keyset().unwrap();
    let app = build_router(state);

    let pk_body: RegistryKeyResponse = read_json(
        app.clone()
            .oneshot(get("/api/v1/registry-key"))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let pubkey = parse_registry_pubkey(&pk_body.public_key);

    let (mint_state, mint_req) = mint_start(&keyset, &mint_ctx()).unwrap();
    let mint_response: MintResponse = read_json(
        app.clone()
            .oneshot(post_json("/api/v1/mints", &mint_req))
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    let credential = mint_finish(mint_state, mint_response).unwrap();
    let payload = publish(
        &credential,
        make_review_body("ok", 5),
        DisclosureMask::default(),
    )
    .unwrap();
    let resp = app
        .clone()
        .oneshot(post_json("/api/v1/reviews", &payload))
        .await
        .unwrap();
    let created: ReviewWithProofAndSth = read_json(resp.into_body()).await;
    created.sth.verify(&pubkey).unwrap();

    // Pretend the payload was modified locally: leaf hash changes,
    // so the original inclusion proof no longer matches the STH root.
    let mut tampered = created.payload.clone();
    tampered.review_body.text = "tampered locally".into();
    let bad_leaf = leaf_hash(&tampered).unwrap();
    assert!(
        created
            .inclusion_proof
            .verify(&bad_leaf, &created.sth.root_hash)
            .is_err()
    );
}
