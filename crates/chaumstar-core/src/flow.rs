//! End-to-end flow helpers: `mint_start`, `mint_finish`, `publish`, `verify`.

use ed25519_dalek::{Signature as EdSig, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::attrs::{DisclosureMask, MintContext};
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
    ctx: &MintContext,
) -> Result<(MintState, MintRequest), Error> {
    let mut rng = OsRng;
    let hsk = SigningKey::generate(&mut rng);
    let hpk = hsk.verifying_key();
    let hpk_bytes = hpk.to_bytes();

    let committed_messages: Vec<Vec<u8>> = vec![hpk_bytes.to_vec()];
    let (commitment, blind_factor) = Commitment::<BBSplus<Cs>>::commit(Some(&committed_messages))?;

    let mint_request = MintRequest {
        issuer_id: keyset.issuer_id.clone(),
        merchant_id: ctx.merchant_id.clone(),
        purchase_tier: ctx.purchase_tier,
        product_category: ctx.product_category,
        keyset_id: keyset.keyset_id.clone(),
        commitment_bytes: commitment.to_bytes(),
    };

    let mint_state = MintState {
        hsk_bytes: hsk.to_bytes(),
        hpk_bytes,
        blind_factor_bytes: blind_factor.to_bytes(),
        keyset: keyset.clone(),
        merchant_id: ctx.merchant_id.clone(),
        purchase_tier: ctx.purchase_tier,
        product_category: ctx.product_category,
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
        state.purchase_tier.as_bytes().to_vec(),
        state.product_category.as_bytes().to_vec(),
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
        purchase_tier: state.purchase_tier,
        product_category: state.product_category,
    })
}

/// Publish a review using a credential. Generates the BBS+ presentation proof
/// and the Ed25519 holder signature, both bound to the canonical review body.
/// The `mask` controls which of the two reviewer-controlled attributes are
/// disclosed in the published payload.
pub fn publish(
    credential: &Credential,
    body: ReviewBody,
    mask: DisclosureMask,
) -> Result<ReviewPayload, Error> {
    let disclosed_tier = mask.disclose_tier.then_some(credential.purchase_tier);
    let disclosed_category = mask
        .disclose_category
        .then_some(credential.product_category);

    let m_jcs = canonical_message(
        &body,
        &credential.hpk,
        &credential.keyset.keyset_id,
        disclosed_tier,
        disclosed_category,
    )?;

    // Ed25519 sign the canonical message.
    let hsk = SigningKey::from_bytes(&credential.hsk);
    let sig_ed = hsk.sign(&m_jcs);

    // BBS+ presentation proof, bound to the same canonical message.
    let pk = credential.keyset.pubkey()?;
    let blind_factor = BlindFactor::from_bytes(&credential.blind_factor)?;
    let presentation_header = Sha256::digest(&m_jcs).to_vec();

    let revealed_messages: Vec<Vec<u8>> = vec![
        credential.merchant_id.as_bytes().to_vec(),
        credential.purchase_tier.as_bytes().to_vec(),
        credential.product_category.as_bytes().to_vec(),
    ];
    let committed_messages: Vec<Vec<u8>> = vec![credential.hpk.to_vec()];

    let disclosed_indexes = build_disclosed_indexes(mask);
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
            purchase_tier: disclosed_tier,
            product_category: disclosed_category,
        },
        sig: sig_ed.to_bytes(),
    })
}

/// Verify the cryptographic parts of a [`ReviewPayload`] (Ed25519 sig, BBS+
/// presentation proof, canonical-message binding). Does NOT consult any
/// nullifier registry — use [`verify`] for that.
pub fn verify_proof_only(
    payload: &ReviewPayload,
    keyset: &PublicKeyset,
) -> Result<(), VerifyError> {
    if payload.credential_proof.keyset_id != keyset.keyset_id {
        return Err(VerifyError::KeysetMismatch);
    }

    let disclosed_tier = payload.credential_proof.purchase_tier;
    let disclosed_category = payload.credential_proof.product_category;

    let m_jcs = canonical_message(
        &payload.review_body,
        &payload.credential_proof.hpk,
        &keyset.keyset_id,
        disclosed_tier,
        disclosed_category,
    )
    .map_err(|e| VerifyError::Malformed(format!("{e}")))?;

    let hpk = VerifyingKey::from_bytes(&payload.credential_proof.hpk)
        .map_err(|_| VerifyError::HolderSignatureInvalid)?;
    let ed_sig = EdSig::from_bytes(&payload.sig);
    hpk.verify(&m_jcs, &ed_sig)
        .map_err(|_| VerifyError::HolderSignatureInvalid)?;

    let pk = keyset
        .pubkey()
        .map_err(|_| VerifyError::Malformed("invalid keyset public key".into()))?;
    let proof = PoKSignature::<BBSplus<Cs>>::from_bytes(&payload.credential_proof.bbs_proof)
        .map_err(|_| VerifyError::ProofInvalid)?;

    let presentation_header = Sha256::digest(&m_jcs).to_vec();

    // Rebuild disclosed_messages in the canonical [merchant, tier?, category?] order.
    let mut disclosed_messages: Vec<Vec<u8>> =
        vec![payload.review_body.merchant_id.as_bytes().to_vec()];
    if let Some(t) = disclosed_tier {
        disclosed_messages.push(t.as_bytes().to_vec());
    }
    if let Some(c) = disclosed_category {
        disclosed_messages.push(c.as_bytes().to_vec());
    }
    let disclosed_committed_messages: Vec<Vec<u8>> = vec![payload.credential_proof.hpk.to_vec()];

    let disclosed_indexes =
        build_disclosed_indexes_from_options(disclosed_tier, disclosed_category);
    let disclosed_commitment_indexes: [usize; 1] = [0];
    // Total message vector length is fixed at 3 (merchant, tier, category).
    let total_messages = 3usize;

    proof
        .blind_proof_verify(
            &pk,
            Some(HEADER),
            Some(&presentation_header),
            Some(total_messages),
            Some(&disclosed_messages),
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

fn build_disclosed_indexes(mask: DisclosureMask) -> Vec<usize> {
    build_disclosed_indexes_from_options(
        mask.disclose_tier.then_some(()),
        mask.disclose_category.then_some(()),
    )
    .to_vec()
}

fn build_disclosed_indexes_from_options<A, B>(tier: Option<A>, category: Option<B>) -> Vec<usize> {
    let mut out = vec![0usize];
    if tier.is_some() {
        out.push(1);
    }
    if category.is_some() {
        out.push(2);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::PurchaseTier;

    #[test]
    fn disclosed_indexes_for_each_mask() {
        assert_eq!(build_disclosed_indexes(DisclosureMask::default()), vec![0]);
        assert_eq!(
            build_disclosed_indexes(DisclosureMask {
                disclose_tier: true,
                disclose_category: false
            }),
            vec![0, 1]
        );
        assert_eq!(
            build_disclosed_indexes(DisclosureMask {
                disclose_tier: false,
                disclose_category: true
            }),
            vec![0, 2]
        );
        assert_eq!(
            build_disclosed_indexes(DisclosureMask {
                disclose_tier: true,
                disclose_category: true
            }),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn disclosed_indexes_from_options_match_mask() {
        assert_eq!(
            build_disclosed_indexes_from_options::<PurchaseTier, &str>(None, None),
            vec![0]
        );
        assert_eq!(
            build_disclosed_indexes_from_options(Some(PurchaseTier::Mid), Some("x")),
            vec![0, 1, 2]
        );
    }
}
