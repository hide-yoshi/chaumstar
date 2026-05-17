//! Review payload types — the data published to the registry / reader.

use serde::{Deserialize, Serialize};

use crate::attrs::{ProductCategory, PurchaseTier};
use crate::error::Error;
use crate::keyset::{KeysetId, hex_bytes_32, hex_bytes_64, hex_vec};

/// The user-visible review content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewBody {
    pub text: String,
    pub rating: u8,
    pub merchant_id: String,
    pub issuer_id: String,
    pub issued_at: String,
    pub timestamp: String,
}

/// The cryptographic proof portion of a [`ReviewPayload`].
///
/// `purchase_tier` / `product_category` follow the BBS+ disclosure semantics:
/// `None` ⇒ the attribute was kept hidden in the proof; `Some(v)` ⇒ the proof
/// discloses that attribute at value `v`, and verification will fail unless
/// the BBS+ proof was generated with the same disclosure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProof {
    #[serde(with = "hex_bytes_32")]
    pub hpk: [u8; 32],
    pub keyset_id: KeysetId,
    #[serde(with = "hex_vec")]
    pub bbs_proof: Vec<u8>,
    #[serde(default)]
    pub purchase_tier: Option<PurchaseTier>,
    #[serde(default)]
    pub product_category: Option<ProductCategory>,
}

/// The full payload broadcast to a registry / read by a reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPayload {
    pub version: String,
    pub review_body: ReviewBody,
    pub credential_proof: CredentialProof,
    #[serde(with = "hex_bytes_64")]
    pub sig: [u8; 64],
}

/// Build the canonical message `M` that gets:
/// 1. Signed with Ed25519 by the holder, and
/// 2. SHA-256 hashed to form the BBS+ `presentation_header`.
///
/// `disclosed_tier` / `disclosed_category` are the values that will appear in
/// `CredentialProof.purchase_tier` / `.product_category`. Passing `None` means
/// the attribute is hidden in this presentation.
pub(crate) fn canonical_message(
    body: &ReviewBody,
    hpk: &[u8; 32],
    keyset_id: &KeysetId,
    disclosed_tier: Option<PurchaseTier>,
    disclosed_category: Option<ProductCategory>,
) -> Result<Vec<u8>, Error> {
    let value = serde_json::json!({
        "v": crate::PROTOCOL_VERSION,
        "type": "review",
        "review_body": {
            "text": body.text,
            "rating": body.rating,
            "merchant_id": body.merchant_id,
            "issuer_id": body.issuer_id,
            "issued_at": body.issued_at,
            "timestamp": body.timestamp,
        },
        "credential": {
            "hpk": hex::encode(hpk),
            "keyset_id": keyset_id.to_hex(),
        },
        "disclosure": {
            "purchase_tier":    disclosed_tier.map(|t| t.as_str()),
            "product_category": disclosed_category.map(|c| c.as_str()),
        }
    });
    serde_jcs::to_vec(&value).map_err(|e| Error::Jcs(format!("{e}")))
}
