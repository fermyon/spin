// TODO: there's nothing key-value-specific about this utility, so it could be moved elsewhere, e.g. to a utility
// crate of some kind.

use std::collections::HashMap;

/// This is a table for generating unique u32 identifiers for each element in a dynamically-changing set of
/// resources.
///
/// This is inspired by the `Table` type in
/// [wasi-common](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-common) and serves the same
/// purpose: allow opaque resources and their lifetimes to be managed across an interface boundary, analogous to
/// how file handles work across the user-kernel boundary.
///
pub struct Table<V> {
    next_key: u32,
    tuples: HashMap<u32, V>,
}

impl<V> Table<V> {
    /// Create a new, empty table.
    pub fn new() -> Self {
        Self {
            next_key: 0,
            tuples: HashMap::new(),
        }
    }

    /// Add the specified resource to this table.
    ///
    /// If the table is full (i.e. there already are 2^32 resources inside), this returns `Err(())`.  Otherwise, a
    /// new, unique identifier is allocated for the resource and returned.
    ///
    /// This function will attempt to avoid reusing recently closed identifiers, but after 2^32 calls to this
    /// function they will start repeating.
    pub fn push(&mut self, value: V) -> Result<u32, ()> {
        if self.tuples.len() == u32::MAX as usize {
            Err(())
        } else {
            loop {
                let key = self.next_key;
                self.next_key = self.next_key.wrapping_add(1);
                if self.tuples.contains_key(&key) {
                    continue;
                }
                self.tuples.insert(key, value);
                return Ok(key);
            }
        }
    }

    /// Get a reference to the resource identified by the specified `key`, if it exists.
    pub fn get(&self, key: u32) -> Option<&V> {
        self.tuples.get(&key)
    }

    /// Remove the resource identified by the specified `key`, if present.
    ///
    /// This makes the key eligible for eventual reuse (i.e. for a newly-pushed resource).
    pub fn remove(&mut self, key: u32) -> Option<V> {
        self.tuples.remove(&key)
    }
}
