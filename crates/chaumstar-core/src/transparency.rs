//! RFC 6962 (Certificate Transparency) flavored Merkle log primitives.
//!
//! Only the pieces needed by chaumstar v0.3 are implemented:
//!   * `leaf_hash` / `node_hash`
//!   * append-only Merkle root over a slice of leaves
//!   * `InclusionProof` generation + verification
//!   * `Sth` (Signed Tree Head) construction + verification
//!
//! Out of scope (defer to v1+): consistency proofs between two STHs,
//! non-inclusion proofs, witness cosigning.

use ed25519_dalek::{Signature as EdSig, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, VerifyError};
use crate::keyset::{hex_bytes_32, hex_bytes_64};
use crate::payload::ReviewPayload;

/// SHA-256 digest of a chaumstar Merkle log leaf.
///
/// Following RFC 6962 §2.1, leaves are prefixed with 0x00 before hashing.
pub fn leaf_hash(payload: &ReviewPayload) -> Result<[u8; 32], Error> {
    let bytes =
        serde_jcs::to_vec(payload).map_err(|e| Error::Jcs(format!("leaf_hash JCS: {e}")))?;
    Ok(leaf_hash_bytes(&bytes))
}

/// Lower-level leaf hash for callers that already have canonical bytes.
pub fn leaf_hash_bytes(canonical_payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x00]);
    hasher.update(canonical_payload);
    hasher.finalize().into()
}

/// RFC 6962 §2.1 internal node hash.
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Compute the Merkle tree root over a sequence of leaf hashes
/// (RFC 6962 §2.1). The empty tree's root is `SHA-256("")`.
pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return Sha256::digest(b"").into();
    }
    if leaves.len() == 1 {
        return leaves[0];
    }
    let k = largest_power_of_two_lt(leaves.len());
    let left = merkle_root(&leaves[..k]);
    let right = merkle_root(&leaves[k..]);
    node_hash(&left, &right)
}

/// Largest power of two strictly less than `n` (n ≥ 2). Per RFC 6962 §2.1.
fn largest_power_of_two_lt(n: usize) -> usize {
    let mut k: usize = 1;
    while k * 2 < n {
        k *= 2;
    }
    k
}

/// Inclusion proof for a single leaf (RFC 6962 §2.1.1). `path` lists the
/// sibling hashes from leaf level up; `tree_size` tells the verifier how to
/// walk the structure (since trees can be unbalanced).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProof {
    pub leaf_index: u64,
    pub tree_size: u64,
    pub path: Vec<HashHex>,
}

/// Wrapper so paths serialize as hex strings rather than `[u8; 32]` arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HashHex(#[serde(with = "hex_bytes_32")] pub [u8; 32]);

impl From<[u8; 32]> for HashHex {
    fn from(h: [u8; 32]) -> Self {
        Self(h)
    }
}

impl InclusionProof {
    /// Generate an inclusion proof for `leaf_index` against `leaves`.
    pub fn build(leaves: &[[u8; 32]], leaf_index: u64) -> Result<Self, Error> {
        let n = leaves.len();
        let idx = leaf_index as usize;
        if idx >= n {
            return Err(Error::InvalidInput(format!(
                "leaf_index {leaf_index} out of range (tree_size {n})"
            )));
        }
        let path = path_for(leaves, idx);
        Ok(Self {
            leaf_index,
            tree_size: n as u64,
            path: path.into_iter().map(HashHex).collect(),
        })
    }

    /// Verify that `leaf` is included in a Merkle tree whose root is `root`.
    pub fn verify(&self, leaf: &[u8; 32], root: &[u8; 32]) -> Result<(), VerifyError> {
        let recomputed = compute_root_from_path(
            leaf,
            self.leaf_index,
            self.tree_size,
            &self.path.iter().map(|h| h.0).collect::<Vec<_>>(),
        )
        .map_err(|e| VerifyError::Malformed(format!("inclusion proof: {e}")))?;
        if &recomputed != root {
            return Err(VerifyError::ProofInvalid);
        }
        Ok(())
    }
}

fn path_for(leaves: &[[u8; 32]], idx: usize) -> Vec<[u8; 32]> {
    let n = leaves.len();
    if n == 1 {
        return vec![];
    }
    let k = largest_power_of_two_lt(n);
    if idx < k {
        let mut out = path_for(&leaves[..k], idx);
        out.push(merkle_root(&leaves[k..]));
        out
    } else {
        let mut out = path_for(&leaves[k..], idx - k);
        out.push(merkle_root(&leaves[..k]));
        out
    }
}

fn compute_root_from_path(
    leaf: &[u8; 32],
    leaf_index: u64,
    tree_size: u64,
    path: &[[u8; 32]],
) -> Result<[u8; 32], String> {
    if leaf_index >= tree_size {
        return Err(format!("leaf_index {leaf_index} >= tree_size {tree_size}"));
    }
    // Mirror RFC 6962 §2.1.1 PATH reconstruction.
    let mut fn_idx = leaf_index;
    let mut sn = tree_size - 1;
    let mut hash = *leaf;
    let mut iter = path.iter();
    while sn != 0 {
        let sibling = iter
            .next()
            .ok_or_else(|| "path shorter than expected".to_string())?;
        if fn_idx & 1 == 1 || fn_idx == sn {
            // sibling is on the left
            hash = node_hash(sibling, &hash);
            if fn_idx & 1 == 0 {
                while fn_idx & 1 == 0 {
                    fn_idx >>= 1;
                    sn >>= 1;
                }
            }
        } else {
            // sibling is on the right
            hash = node_hash(&hash, sibling);
        }
        fn_idx >>= 1;
        sn >>= 1;
    }
    if iter.next().is_some() {
        return Err("path longer than expected".into());
    }
    Ok(hash)
}

/// Signed Tree Head — Registry's commitment to its log at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sth {
    pub tree_size: u64,
    #[serde(with = "hex_bytes_32")]
    pub root_hash: [u8; 32],
    pub timestamp: String,
    #[serde(with = "hex_bytes_64")]
    pub sig: [u8; 64],
}

impl Sth {
    /// Construct + sign a fresh STH.
    pub fn new(
        sk: &SigningKey,
        tree_size: u64,
        root_hash: [u8; 32],
        timestamp: String,
    ) -> Result<Self, Error> {
        let msg = sth_canonical_bytes(tree_size, &root_hash, &timestamp)?;
        let sig = sk.sign(&msg).to_bytes();
        Ok(Self {
            tree_size,
            root_hash,
            timestamp,
            sig,
        })
    }

    /// Verify the STH against the Registry public key.
    pub fn verify(&self, registry_pk: &[u8; 32]) -> Result<(), VerifyError> {
        let pk = VerifyingKey::from_bytes(registry_pk)
            .map_err(|_| VerifyError::Malformed("registry pubkey invalid".into()))?;
        let msg = sth_canonical_bytes(self.tree_size, &self.root_hash, &self.timestamp)
            .map_err(|e| VerifyError::Malformed(format!("sth canonical: {e}")))?;
        let sig = EdSig::from_bytes(&self.sig);
        pk.verify(&msg, &sig)
            .map_err(|_| VerifyError::HolderSignatureInvalid)?;
        Ok(())
    }
}

fn sth_canonical_bytes(
    tree_size: u64,
    root_hash: &[u8; 32],
    timestamp: &str,
) -> Result<Vec<u8>, Error> {
    let v = serde_json::json!({
        "v": crate::PROTOCOL_VERSION,
        "type": "sth",
        "tree_size": tree_size,
        "root_hash": hex::encode(root_hash),
        "timestamp": timestamp,
    });
    serde_jcs::to_vec(&v).map_err(|e| Error::Jcs(format!("{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn h(b: u8) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[0] = b;
        out
    }

    #[test]
    fn merkle_root_single_leaf_is_that_leaf() {
        let leaf = h(0xaa);
        assert_eq!(merkle_root(&[leaf]), leaf);
    }

    #[test]
    fn merkle_root_two_leaves_is_node_hash() {
        let a = h(1);
        let b = h(2);
        assert_eq!(merkle_root(&[a, b]), node_hash(&a, &b));
    }

    #[test]
    fn merkle_root_unbalanced_three_leaves() {
        // Layout per RFC 6962:
        //         root
        //        /    \
        //      h01     2
        //      / \
        //     0   1
        let leaves = [h(0), h(1), h(2)];
        let h01 = node_hash(&leaves[0], &leaves[1]);
        let expected = node_hash(&h01, &leaves[2]);
        assert_eq!(merkle_root(&leaves), expected);
    }

    #[test]
    fn inclusion_proof_round_trip_for_every_index() {
        for size in 1..=12 {
            let leaves: Vec<[u8; 32]> = (0..size as u8).map(h).collect();
            let root = merkle_root(&leaves);
            for i in 0..size {
                let proof = InclusionProof::build(&leaves, i as u64).unwrap();
                proof
                    .verify(&leaves[i], &root)
                    .unwrap_or_else(|_| panic!("size={size}, i={i}"));
            }
        }
    }

    #[test]
    fn inclusion_proof_against_wrong_root_fails() {
        let leaves: Vec<[u8; 32]> = (0..4).map(h).collect();
        let proof = InclusionProof::build(&leaves, 1).unwrap();
        let mut wrong_root = merkle_root(&leaves);
        wrong_root[0] ^= 1;
        assert!(proof.verify(&leaves[1], &wrong_root).is_err());
    }

    #[test]
    fn inclusion_proof_for_wrong_leaf_fails() {
        let leaves: Vec<[u8; 32]> = (0..4).map(h).collect();
        let root = merkle_root(&leaves);
        let proof = InclusionProof::build(&leaves, 1).unwrap();
        // Try to claim leaf 2 is at index 1
        assert!(proof.verify(&leaves[2], &root).is_err());
    }

    #[test]
    fn inclusion_proof_out_of_range_errors() {
        let leaves: Vec<[u8; 32]> = (0..4).map(h).collect();
        assert!(InclusionProof::build(&leaves, 4).is_err());
    }

    #[test]
    fn sth_signature_round_trip() {
        let sk = SigningKey::generate(&mut OsRng);
        let pk_bytes = sk.verifying_key().to_bytes();

        let sth = Sth::new(&sk, 7, h(0x42), "2026-05-17T10:00:00Z".into()).unwrap();
        sth.verify(&pk_bytes).expect("valid sth verifies");
    }

    #[test]
    fn sth_signature_against_wrong_key_fails() {
        let sk = SigningKey::generate(&mut OsRng);
        let other = SigningKey::generate(&mut OsRng);
        let sth = Sth::new(&sk, 1, h(0xff), "2026-05-17T10:00:00Z".into()).unwrap();
        assert!(sth.verify(&other.verifying_key().to_bytes()).is_err());
    }

    #[test]
    fn sth_tampering_breaks_signature() {
        let sk = SigningKey::generate(&mut OsRng);
        let pk_bytes = sk.verifying_key().to_bytes();
        let mut sth = Sth::new(&sk, 3, h(0x10), "2026-05-17T10:00:00Z".into()).unwrap();
        // Tamper any signed field.
        sth.tree_size = 99;
        assert!(sth.verify(&pk_bytes).is_err());
    }
}
