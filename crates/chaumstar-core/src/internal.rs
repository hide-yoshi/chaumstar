//! Internal zkryptium type aliases and protocol constants. Kept here so the
//! rest of the crate doesn't have to repeat the long generic types.

pub(crate) use zkryptium::{
    bbsplus::{commitment::BlindFactor, keys::BBSplusPublicKey},
    keys::pair::KeyPair,
    schemes::{
        algorithms::{BBSplus, BbsBls12381Sha256, Scheme},
        generics::{BlindSignature, Commitment, PoKSignature},
    },
};

pub(crate) type Suite = BbsBls12381Sha256;
pub(crate) type Cs = <Suite as Scheme>::Ciphersuite;
pub(crate) type IssuerKeypair = KeyPair<BBSplus<Cs>>;

/// Stable BBS+ `header` for chaumstar v0. This value is signed together with
/// the message vector at mint time and re-supplied at verify time. It binds
/// every credential to the chaumstar protocol version.
pub(crate) const HEADER: &[u8] = b"chaumstar/0.1";

/// Fixed-size byte length of a BBS+ blind signature for the chosen suite.
pub(crate) const BBS_SIGNATURE_BYTES: usize = 80;
