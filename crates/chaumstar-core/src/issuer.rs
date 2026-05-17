//! Issuer (mint authority) implementation. Holds the BBS+ secret key for one
//! `(issuer_id, merchant_id)` pair.

use rand::RngCore;

use crate::credential::{MintRequest, MintResponse};
use crate::error::Error;
use crate::internal::{BBSplus, BlindSignature, Cs, HEADER, IssuerKeypair};
use crate::keyset::{KeysetId, PublicKeyset};

pub struct Issuer {
    issuer_id: String,
    merchant_id: String,
    keypair: IssuerKeypair,
    keyset_id: KeysetId,
}

impl Issuer {
    /// Generate a fresh random keypair for `(issuer_id, merchant_id)`.
    pub fn generate(issuer_id: &str, merchant_id: &str) -> Self {
        let mut rng = rand::rngs::OsRng;
        let mut ikm = vec![0u8; 32];
        rng.fill_bytes(&mut ikm);
        let keypair = IssuerKeypair::generate(&ikm, None, None)
            .expect("BBS+ keygen with 32B IKM should not fail");
        let keyset_id = KeysetId::from_pubkey_bytes(&keypair.public_key().to_bytes());
        Self {
            issuer_id: issuer_id.to_string(),
            merchant_id: merchant_id.to_string(),
            keypair,
            keyset_id,
        }
    }

    pub fn issuer_id(&self) -> &str {
        &self.issuer_id
    }

    pub fn merchant_id(&self) -> &str {
        &self.merchant_id
    }

    pub fn public_keyset(&self) -> PublicKeyset {
        PublicKeyset {
            issuer_id: self.issuer_id.clone(),
            merchant_id: self.merchant_id.clone(),
            keyset_id: self.keyset_id.clone(),
            public_key_bytes: self.keypair.public_key().to_bytes().to_vec(),
        }
    }

    /// Blind-sign a mint request from a wallet. Validates that the request
    /// targets this issuer/merchant, then runs BBS+ blind signing over
    /// `(hpk_commitment, merchant_id, issued_at, purchase_tier, product_category)`.
    pub fn blind_sign(&self, request: &MintRequest) -> Result<MintResponse, Error> {
        if request.issuer_id != self.issuer_id {
            return Err(Error::InvalidInput(format!(
                "issuer_id mismatch: expected {}, got {}",
                self.issuer_id, request.issuer_id
            )));
        }
        if request.merchant_id != self.merchant_id {
            return Err(Error::InvalidInput(format!(
                "merchant_id mismatch: expected {}, got {}",
                self.merchant_id, request.merchant_id
            )));
        }
        if request.keyset_id != self.keyset_id {
            return Err(Error::InvalidInput("keyset_id mismatch".into()));
        }

        let revealed_messages: Vec<Vec<u8>> = vec![
            request.merchant_id.as_bytes().to_vec(),
            request.issued_at.as_bytes().to_vec(),
            request.purchase_tier.as_bytes().to_vec(),
            request.product_category.as_bytes().to_vec(),
        ];

        let blind_sig = BlindSignature::<BBSplus<Cs>>::blind_sign(
            self.keypair.private_key(),
            self.keypair.public_key(),
            Some(&request.commitment_bytes),
            Some(HEADER),
            Some(&revealed_messages),
        )?;

        Ok(MintResponse {
            blind_signature_bytes: blind_sig.to_bytes().to_vec(),
        })
    }
}
