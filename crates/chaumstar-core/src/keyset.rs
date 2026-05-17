//! Issuer keyset types.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::Error;
use crate::internal::BBSplusPublicKey;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeysetId(#[serde(with = "hex_bytes_8")] pub [u8; 8]);

impl KeysetId {
    pub fn from_pubkey_bytes(pk: &[u8]) -> Self {
        let mut id = [0u8; 8];
        id.copy_from_slice(&Sha256::digest(pk)[..8]);
        Self(id)
    }

    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// Public, transmittable description of an issuer keyset for a single merchant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyset {
    pub issuer_id: String,
    pub merchant_id: String,
    pub keyset_id: KeysetId,
    /// Compressed BLS12-381 G₂ point bytes (96 B).
    #[serde(with = "hex_vec")]
    pub public_key_bytes: Vec<u8>,
}

impl PublicKeyset {
    pub(crate) fn pubkey(&self) -> Result<BBSplusPublicKey, Error> {
        BBSplusPublicKey::from_bytes(&self.public_key_bytes)
            .map_err(|e| Error::Bbs(format!("PublicKey::from_bytes: {e:?}")))
    }
}

pub(crate) mod hex_bytes_8 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 8], D::Error> {
        let s = String::deserialize(d)?;
        let v = hex::decode(s).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 8 bytes"))
    }
}

pub(crate) mod hex_bytes_32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(d)?;
        let v = hex::decode(s).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}

pub(crate) mod hex_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let s = String::deserialize(d)?;
        let v = hex::decode(s).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

pub(crate) mod hex_vec {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        hex::decode(s).map_err(serde::de::Error::custom)
    }
}
