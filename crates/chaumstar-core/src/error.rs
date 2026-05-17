//! Error types for chaumstar-core.

use thiserror::Error;

/// Errors raised by mint / publish / wallet-side operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("BBS+ error: {0}")]
    Bbs(String),

    #[error("Ed25519 error: {0}")]
    Ed25519(String),

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("canonical serialization error: {0}")]
    Jcs(String),
}

impl From<zkryptium::errors::Error> for Error {
    fn from(e: zkryptium::errors::Error) -> Self {
        Error::Bbs(format!("{e:?}"))
    }
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        Error::Ed25519(format!("{e}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Jcs(format!("{e}"))
    }
}

/// Errors raised by [`crate::verify`].
#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("BBS+ proof is invalid")]
    ProofInvalid,

    #[error("Ed25519 holder signature is invalid")]
    HolderSignatureInvalid,

    #[error("keyset does not match the credential")]
    KeysetMismatch,

    #[error("credential has already been used (double-review)")]
    AlreadyUsed,

    #[error("malformed payload: {0}")]
    Malformed(String),
}
