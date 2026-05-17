//! Shared application state — in-memory store of issuers and reviews,
//! plus the Registry's append-only Merkle transparency log.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use chaumstar_core::{
    InclusionProof, Issuer, KeysetId, MemoryRegistry, PublicKeyset, ReviewPayload, Sth,
    VerifyError, leaf_hash, transparency, verify,
};
use chrono::SecondsFormat;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

#[derive(Clone)]
pub struct AppState {
    issuers: Arc<RwLock<HashMap<KeysetId, Arc<Issuer>>>>,
    reviews: Arc<ReviewStore>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            issuers: Arc::new(RwLock::new(HashMap::new())),
            reviews: Arc::new(ReviewStore::new()),
        }
    }

    /// Register an issuer with the server. Idempotent on `keyset_id`.
    pub fn register_issuer(&self, issuer: Issuer) {
        let kid = issuer.public_keyset().keyset_id;
        self.issuers.write().unwrap().insert(kid, Arc::new(issuer));
    }

    /// Convenience helper used by tests and bootstrap code.
    pub fn first_public_keyset(&self) -> Option<PublicKeyset> {
        self.issuers
            .read()
            .unwrap()
            .values()
            .next()
            .map(|i| i.public_keyset())
    }

    pub fn list_keysets(&self) -> Vec<PublicKeyset> {
        self.issuers
            .read()
            .unwrap()
            .values()
            .map(|i| i.public_keyset())
            .collect()
    }

    pub fn get_issuer(&self, kid: &KeysetId) -> Option<Arc<Issuer>> {
        self.issuers.read().unwrap().get(kid).cloned()
    }

    pub fn reviews(&self) -> Arc<ReviewStore> {
        self.reviews.clone()
    }
}

pub struct ReviewStore {
    inner: Mutex<ReviewStoreInner>,
    signing_key: SigningKey,
    registry_pubkey: [u8; 32],
}

#[derive(Default)]
struct ReviewStoreInner {
    by_hpk: HashMap<[u8; 32], (ReviewPayload, u64)>,
    order: Vec<[u8; 32]>,
    leaves: Vec<[u8; 32]>,
}

#[derive(Debug)]
pub enum InsertError {
    AlreadyUsed,
    Verify(VerifyError),
    Internal(String),
}

impl Default for ReviewStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewStore {
    pub fn new() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let registry_pubkey = signing_key.verifying_key().to_bytes();
        Self {
            inner: Mutex::new(ReviewStoreInner::default()),
            signing_key,
            registry_pubkey,
        }
    }

    pub fn registry_public_key(&self) -> [u8; 32] {
        self.registry_pubkey
    }

    /// Compute the current Signed Tree Head.
    pub fn current_sth(&self) -> Result<Sth, InsertError> {
        let inner = self.inner.lock().unwrap();
        self.sth_locked(&inner)
    }

    fn sth_locked(&self, inner: &ReviewStoreInner) -> Result<Sth, InsertError> {
        let root = transparency::merkle_root(&inner.leaves);
        let ts = now_iso();
        Sth::new(&self.signing_key, inner.leaves.len() as u64, root, ts)
            .map_err(|e| InsertError::Internal(format!("sth: {e}")))
    }

    /// Atomically verify a payload against `keyset`, insert it into the log,
    /// and return the inclusion proof + the post-insert STH.
    pub fn check_and_insert(
        &self,
        payload: ReviewPayload,
        keyset: &PublicKeyset,
    ) -> Result<(ReviewPayload, InclusionProof, Sth), InsertError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.by_hpk.contains_key(&payload.credential_proof.hpk) {
            return Err(InsertError::AlreadyUsed);
        }
        let mut throwaway = MemoryRegistry::default();
        verify(&payload, keyset, &mut throwaway).map_err(InsertError::Verify)?;

        let leaf = leaf_hash(&payload).map_err(|e| InsertError::Internal(format!("leaf: {e}")))?;
        let hpk = payload.credential_proof.hpk;
        let idx = inner.leaves.len() as u64;
        inner.leaves.push(leaf);
        inner.order.push(hpk);
        inner.by_hpk.insert(hpk, (payload.clone(), idx));

        let proof = InclusionProof::build(&inner.leaves, idx)
            .map_err(|e| InsertError::Internal(format!("inclusion: {e}")))?;
        let sth = self.sth_locked(&inner)?;
        Ok((payload, proof, sth))
    }

    pub fn list_with_proofs(
        &self,
    ) -> Result<(Vec<(ReviewPayload, InclusionProof)>, Sth), InsertError> {
        let inner = self.inner.lock().unwrap();
        let mut result = Vec::with_capacity(inner.order.len());
        for hpk in &inner.order {
            if let Some((payload, idx)) = inner.by_hpk.get(hpk) {
                let proof = InclusionProof::build(&inner.leaves, *idx)
                    .map_err(|e| InsertError::Internal(format!("inclusion: {e}")))?;
                result.push((payload.clone(), proof));
            }
        }
        let sth = self.sth_locked(&inner)?;
        Ok((result, sth))
    }

    pub fn get_with_proof(
        &self,
        hpk: &[u8; 32],
    ) -> Result<Option<(ReviewPayload, InclusionProof, Sth)>, InsertError> {
        let inner = self.inner.lock().unwrap();
        let Some((payload, idx)) = inner.by_hpk.get(hpk) else {
            return Ok(None);
        };
        let proof = InclusionProof::build(&inner.leaves, *idx)
            .map_err(|e| InsertError::Internal(format!("inclusion: {e}")))?;
        let sth = self.sth_locked(&inner)?;
        Ok(Some((payload.clone(), proof, sth)))
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
