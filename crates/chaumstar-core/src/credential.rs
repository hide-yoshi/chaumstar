//! Credential and mint-flow data types.

use serde::{Deserialize, Serialize};

use crate::attrs::{ProductCategory, PurchaseTier};
use crate::keyset::{KeysetId, PublicKeyset, hex_bytes_32, hex_vec};

/// Wallet-side state held between `mint_start` and `mint_finish`. Contains the
/// holder's secret material and the BBS+ blinding factor needed to finalize
/// the credential once the issuer's blind signature comes back.
///
/// Serializable so it can cross the WASM/JS boundary, but **never** leave the
/// reviewer's wallet over the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintState {
    #[serde(with = "hex_bytes_32")]
    pub hsk_bytes: [u8; 32],
    #[serde(with = "hex_bytes_32")]
    pub hpk_bytes: [u8; 32],
    #[serde(with = "hex_bytes_32")]
    pub blind_factor_bytes: [u8; 32],
    pub keyset: PublicKeyset,
    pub merchant_id: String,
    pub issued_at: String,
    pub purchase_tier: PurchaseTier,
    pub product_category: ProductCategory,
}

/// Wire-format mint request sent from the reviewer's wallet to the issuer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintRequest {
    pub issuer_id: String,
    pub merchant_id: String,
    pub issued_at: String,
    pub purchase_tier: PurchaseTier,
    pub product_category: ProductCategory,
    pub keyset_id: KeysetId,
    #[serde(with = "hex_vec")]
    pub commitment_bytes: Vec<u8>,
}

/// Wire-format mint response returned from the issuer to the wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintResponse {
    #[serde(with = "hex_vec")]
    pub blind_signature_bytes: Vec<u8>,
}

/// A finished credential ready to be used for publishing a single review.
///
/// **Contains secret material** (`hsk_bytes`, `blind_factor_bytes`) and must
/// only be stored in trusted/encrypted wallet storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub version: String,

    #[serde(with = "hex_bytes_32")]
    pub hpk: [u8; 32],

    /// SECRET. Never publish.
    #[serde(with = "hex_bytes_32")]
    pub hsk: [u8; 32],

    /// SECRET. Needed to generate the BBS+ proof at publish time.
    #[serde(with = "hex_bytes_32")]
    pub blind_factor: [u8; 32],

    /// 80-byte BBS+ blind signature.
    #[serde(with = "hex_vec")]
    pub blind_signature: Vec<u8>,

    pub keyset: PublicKeyset,
    pub merchant_id: String,
    pub issued_at: String,
    pub purchase_tier: PurchaseTier,
    pub product_category: ProductCategory,
}
