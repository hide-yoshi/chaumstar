//! chaumstar-core
//!
//! Core cryptography and protocol types for chaumstar — anonymous verifiable
//! reviews based on BBS+ Anonymous Credentials (BLS12-381-SHA-256 ciphersuite)
//! plus Ed25519 holder signatures.
//!
//! See `PROTOCOL.md`, `CRYPTO.md` in the workspace root for the specification.

pub const PROTOCOL_VERSION: &str = "chaumstar/0.4";

mod attrs;
mod credential;
mod error;
mod flow;
mod internal;
mod issuer;
mod keyset;
mod payload;
mod registry;
pub mod transparency;

pub use attrs::{DisclosureMask, MintContext, ProductCategory, PurchaseTier};
pub use credential::{Credential, MintRequest, MintResponse, MintState};
pub use error::{Error, VerifyError};
pub use flow::{mint_finish, mint_start, publish, verify, verify_proof_only};
pub use issuer::Issuer;
pub use keyset::{KeysetId, PublicKeyset};
pub use payload::{CredentialProof, ReviewBody, ReviewPayload};
pub use registry::{MemoryRegistry, Registry};
pub use transparency::{HashHex, InclusionProof, Sth, leaf_hash, leaf_hash_bytes};
