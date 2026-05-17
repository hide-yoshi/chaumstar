//! Disclosable credential attributes (v0.2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PurchaseTier {
    Low,
    Mid,
    High,
}

impl PurchaseTier {
    /// Bucket a yen amount into a tier using the canonical thresholds.
    pub fn from_yen(amount: u32) -> Self {
        match amount {
            0..=999 => Self::Low,
            1_000..=4_999 => Self::Mid,
            _ => Self::High,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Mid => "mid",
            Self::High => "high",
        }
    }

    pub fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProductCategory {
    Drinks,
    Food,
    Merch,
}

impl ProductCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Drinks => "drinks",
            Self::Food => "food",
            Self::Merch => "merch",
        }
    }

    pub fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }
}

/// Context supplied at mint time. `merchant_id` / `issued_at` are always
/// revealed; `purchase_tier` / `product_category` are attested by the issuer
/// and the reviewer decides at publish time whether to disclose them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintContext {
    pub merchant_id: String,
    pub issued_at: String,
    pub purchase_tier: PurchaseTier,
    pub product_category: ProductCategory,
}

/// Reviewer's per-publish choice of which optional attributes to reveal.
/// The required attributes (hpk, merchant_id, issued_at) are always disclosed.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DisclosureMask {
    #[serde(default)]
    pub disclose_tier: bool,
    #[serde(default)]
    pub disclose_category: bool,
}
