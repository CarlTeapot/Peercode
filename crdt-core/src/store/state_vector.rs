use crate::types::{BlockId, ClientId};
use log::debug;
use std::collections::HashMap;

// StateVector struct to represent the state of seen blocks for each client.
// If sv[client] = N, it means we've seen all blocks from [0, N) clock values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StateVector {
    state_map: HashMap<ClientId, u64>,
}

impl StateVector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the next expected clock for a given client.
    pub fn get(&self, client: &ClientId) -> u64 {
        *self.state_map.get(client).unwrap_or(&0)
    }

    /// Update the state vector after integrating a block.
    pub fn update(&mut self, client: ClientId, end_clock: u64) {
        debug!("Updating state vector for client {}.", client.value);
        let entry = self.state_map.entry(client).or_insert(0);
        *entry = (*entry).max(end_clock);
    }

    /// Returns true if we have seen the given block.
    pub fn has_block(&self, id: &BlockId, len: u64) -> bool {
        self.get(&id.client) >= id.clock.value + len
    }

    /// Returns true if we can integrate the given block
    pub fn can_integrate(&self, id: &BlockId) -> bool {
        let seen = self.get(&id.client);
        seen == id.clock.value
    }
}
