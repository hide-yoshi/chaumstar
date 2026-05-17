//! Shared application state — in-memory store of issuers and reviews.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use chaumstar_core::{
    Issuer, KeysetId, MemoryRegistry, PublicKeyset, ReviewPayload, VerifyError, verify,
};

#[derive(Clone, Default)]
pub struct AppState {
    issuers: Arc<RwLock<HashMap<KeysetId, Arc<Issuer>>>>,
    reviews: Arc<ReviewStore>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
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

#[derive(Default)]
pub struct ReviewStore {
    inner: Mutex<ReviewStoreInner>,
}

#[derive(Default)]
struct ReviewStoreInner {
    by_hpk: HashMap<[u8; 32], ReviewPayload>,
    order: Vec<[u8; 32]>,
}

#[derive(Debug)]
pub enum InsertError {
    AlreadyUsed,
    Verify(VerifyError),
}

impl ReviewStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Atomically verify a payload against `keyset` and insert it.
    /// Holds the store lock for the whole operation to prevent double-spend
    /// races between concurrent submitters.
    pub fn check_and_insert(
        &self,
        payload: ReviewPayload,
        keyset: &PublicKeyset,
    ) -> Result<(), InsertError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.by_hpk.contains_key(&payload.credential_proof.hpk) {
            return Err(InsertError::AlreadyUsed);
        }
        // verify() requires &mut Registry; the store-level lock above is the
        // authoritative duplicate guard, so a throwaway in-process registry
        // is fine here.
        let mut throwaway = MemoryRegistry::default();
        verify(&payload, keyset, &mut throwaway).map_err(InsertError::Verify)?;
        let hpk = payload.credential_proof.hpk;
        inner.order.push(hpk);
        inner.by_hpk.insert(hpk, payload);
        Ok(())
    }

    pub fn list(&self) -> Vec<ReviewPayload> {
        let inner = self.inner.lock().unwrap();
        inner
            .order
            .iter()
            .filter_map(|h| inner.by_hpk.get(h).cloned())
            .collect()
    }

    pub fn get(&self, hpk: &[u8; 32]) -> Option<ReviewPayload> {
        self.inner.lock().unwrap().by_hpk.get(hpk).cloned()
    }
}
