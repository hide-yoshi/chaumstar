//! Nullifier registry: tracks which `hpk` values have already been published.

use std::collections::HashSet;

pub trait Registry {
    fn contains(&self, hpk: &[u8; 32]) -> bool;
    fn insert(&mut self, hpk: [u8; 32]);
}

#[derive(Debug, Default, Clone)]
pub struct MemoryRegistry {
    spent: HashSet<[u8; 32]>,
}

impl MemoryRegistry {
    pub fn len(&self) -> usize {
        self.spent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spent.is_empty()
    }
}

impl Registry for MemoryRegistry {
    fn contains(&self, hpk: &[u8; 32]) -> bool {
        self.spent.contains(hpk)
    }

    fn insert(&mut self, hpk: [u8; 32]) {
        self.spent.insert(hpk);
    }
}
