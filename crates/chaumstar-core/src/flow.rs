//! End-to-end flow helpers: `mint_start`, `mint_finish`, `publish`, `verify`.

use ed25519_dalek::{Signature as EdSig, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::credential::{Credential, MintRequest, MintResponse, MintState};
use crate::error::{Error, VerifyError};
use crate::internal::{
    BBS_SIGNATURE_BYTES, BBSplus, BlindFactor, BlindSignature, Commitment, Cs, HEADER, PoKSignature,
};
use crate::keyset::PublicKeyset;
use crate::payload::{CredentialProof, ReviewBody, ReviewPayload, canonical_message};
use crate::registry::Registry;

/// Begin a mint flow: generate a fresh Ed25519 holder keypair, commit `hpk`
/// to BBS+, and produce a [`MintRequest`] for the issuer.
pub fn mint_start(
    keyset: &PublicKeyset,
    merchant_id: &str,
    issued_at: &str,
) -> Result<(MintState, MintRequest), Error> {
    let mut rng = OsRng;
    let hsk = SigningKey::generate(&mut rng);
    let hpk = hsk.verifying_key();
    let hpk_bytes = hpk.to_bytes();

    let committed_messages: Vec<Vec<u8>> = vec![hpk_bytes.to_vec()];
    let (commitment, blind_factor) = Commitment::<BBSplus<Cs>>::commit(Some(&committed_messages))?;

    let mint_request = MintRequest {
        issuer_id: keyset.issuer_id.clone(),
        merchant_id: merchant_id.to_string(),
        issued_at: issued_at.to_string(),
        keyset_id: keyset.keyset_id.clone(),
        commitment_bytes: commitment.to_bytes(),
    };

    let mint_state = MintState {
        hsk_bytes: hsk.to_bytes(),
        hpk_bytes,
        blind_factor_bytes: blind_factor.to_bytes(),
        keyset: keyset.clone(),
        merchant_id: merchant_id.to_string(),
        issued_at: issued_at.to_string(),
    };

    Ok((mint_state, mint_request))
}

/// Finish a mint flow with the issuer's [`MintResponse`]. Verifies the blind
/// signature locally and packages everything into a [`Credential`].
pub fn mint_finish(state: MintState, response: MintResponse) -> Result<Credential, Error> {
    let sig_bytes: [u8; BBS_SIGNATURE_BYTES] = response
        .blind_signature_bytes
        .as_slice()
        .try_into()
        .map_err(|_| {
            Error::InvalidInput(format!(
                "blind_signature_bytes: expected {} bytes",
                BBS_SIGNATURE_BYTES
            ))
        })?;

    let blind_sig = BlindSignature::<BBSplus<Cs>>::from_bytes(&sig_bytes)?;
    let blind_factor = BlindFactor::from_bytes(&state.blind_factor_bytes)?;
    let pk = state.keyset.pubkey()?;

    let revealed_messages: Vec<Vec<u8>> = vec![
        state.merchant_id.as_bytes().to_vec(),
        state.issued_at.as_bytes().to_vec(),
    ];
    let committed_messages: Vec<Vec<u8>> = vec![state.hpk_bytes.to_vec()];

    blind_sig.verify_blind_sign(
        &pk,
        Some(HEADER),
        Some(&revealed_messages),
        Some(&committed_messages),
        Some(&blind_factor),
    )?;

    Ok(Credential {
        version: crate::PROTOCOL_VERSION.to_string(),
        hpk: state.hpk_bytes,
        hsk: state.hsk_bytes,
        blind_factor: state.blind_factor_bytes,
        blind_signature: sig_bytes.to_vec(),
        keyset: state.keyset,
        merchant_id: state.merchant_id,
        issued_at: state.issued_at,
    })
}

/// Publish a review using a credential. Generates the BBS+ presentation proof
/// and the Ed25519 holder signature, both bound to the canonical review body.
pub fn publish(credential: &Credential, body: ReviewBody) -> Result<ReviewPayload, Error> {
    let m_jcs = canonical_message(&body, &credential.hpk, &credential.keyset.keyset_id)?;

    // Ed25519 sign the canonical message.
    let hsk = SigningKey::from_bytes(&credential.hsk);
    let sig_ed = hsk.sign(&m_jcs);

    // BBS+ presentation proof, bound to the same canonical message.
    let pk = credential.keyset.pubkey()?;
    let blind_factor = BlindFactor::from_bytes(&credential.blind_factor)?;
    let presentation_header = Sha256::digest(&m_jcs).to_vec();

    let revealed_messages: Vec<Vec<u8>> = vec![
        credential.merchant_id.as_bytes().to_vec(),
        credential.issued_at.as_bytes().to_vec(),
    ];
    let committed_messages: Vec<Vec<u8>> = vec![credential.hpk.to_vec()];

    let disclosed_indexes: [usize; 2] = [0, 1];
    let disclosed_commitment_indexes: [usize; 1] = [0];

    let proof = PoKSignature::<BBSplus<Cs>>::blind_proof_gen(
        &pk,
        &credential.blind_signature,
        Some(HEADER),
        Some(&presentation_header),
        Some(&revealed_messages),
        Some(&committed_messages),
        Some(&disclosed_indexes),
        Some(&disclosed_commitment_indexes),
        Some(&blind_factor),
    )?;

    Ok(ReviewPayload {
        version: crate::PROTOCOL_VERSION.to_string(),
        review_body: body,
        credential_proof: CredentialProof {
            hpk: credential.hpk,
            keyset_id: credential.keyset.keyset_id.clone(),
            bbs_proof: proof.to_bytes(),
        },
        sig: sig_ed.to_bytes(),
    })
}

/// Verify the cryptographic parts of a [`ReviewPayload`] (Ed25519 sig,
/// BBS+ presentation proof, canonical-message binding). Does NOT consult any
/// nullifier registry — use [`verify`] for that.
///
/// Suitable for client-side (browser) verification where the reader has the
/// payload and the issuer's public keyset but no shared registry state.
pub fn verify_proof_only(
    payload: &ReviewPayload,
    keyset: &PublicKeyset,
) -> Result<(), VerifyError> {
    if payload.credential_proof.keyset_id != keyset.keyset_id {
        return Err(VerifyError::KeysetMismatch);
    }

    // Recompute the canonical message and verify the Ed25519 holder sig.
    let m_jcs = canonical_message(
        &payload.review_body,
        &payload.credential_proof.hpk,
        &keyset.keyset_id,
    )
    .map_err(|e| VerifyError::Malformed(format!("{e}")))?;

    let hpk = VerifyingKey::from_bytes(&payload.credential_proof.hpk)
        .map_err(|_| VerifyError::HolderSignatureInvalid)?;
    let ed_sig = EdSig::from_bytes(&payload.sig);
    hpk.verify(&m_jcs, &ed_sig)
        .map_err(|_| VerifyError::HolderSignatureInvalid)?;

    // Verify the BBS+ presentation proof, bound to the same canonical message.
    let pk = keyset
        .pubkey()
        .map_err(|_| VerifyError::Malformed("invalid keyset public key".into()))?;
    let proof = PoKSignature::<BBSplus<Cs>>::from_bytes(&payload.credential_proof.bbs_proof)
        .map_err(|_| VerifyError::ProofInvalid)?;

    let presentation_header = Sha256::digest(&m_jcs).to_vec();
    let revealed_messages: Vec<Vec<u8>> = vec![
        payload.review_body.merchant_id.as_bytes().to_vec(),
        payload.review_body.issued_at.as_bytes().to_vec(),
    ];
    let disclosed_committed_messages: Vec<Vec<u8>> = vec![payload.credential_proof.hpk.to_vec()];
    let disclosed_indexes: [usize; 2] = [0, 1];
    let disclosed_commitment_indexes: [usize; 1] = [0];

    proof
        .blind_proof_verify(
            &pk,
            Some(HEADER),
            Some(&presentation_header),
            Some(revealed_messages.len()),
            Some(&revealed_messages),
            Some(&disclosed_committed_messages),
            Some(&disclosed_indexes),
            Some(&disclosed_commitment_indexes),
        )
        .map_err(|_| VerifyError::ProofInvalid)?;

    Ok(())
}

/// Verify a published [`ReviewPayload`] against the issuer's public keyset
/// and append its nullifier to the registry on success.
pub fn verify<R: Registry>(
    payload: &ReviewPayload,
    keyset: &PublicKeyset,
    registry: &mut R,
) -> Result<(), VerifyError> {
    verify_proof_only(payload, keyset)?;

    if registry.contains(&payload.credential_proof.hpk) {
        return Err(VerifyError::AlreadyUsed);
    }
    registry.insert(payload.credential_proof.hpk);
    Ok(())
}
