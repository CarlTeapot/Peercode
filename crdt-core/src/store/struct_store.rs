use crate::store::StateVector;
use crate::structs::Block;

use crate::types::{BlockId, ClientId};
use log::debug;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct StructStore {
    blocks: HashMap<ClientId, Vec<Block>>,
}

impl StructStore {
    pub fn new() -> Self {
        StructStore::default()
    }

    pub fn contains_key(&self, id: &BlockId) -> bool {
        self.get(id).is_some()
    }

    pub fn total_blocks(&self) -> usize {
        self.blocks.values().map(|v| v.len()).sum()
    }

    pub fn insert(&mut self, block: Block) {
        debug!("inserting block {:?}", block);
        let list = self.blocks.entry(block.id.client).or_default();
        let pos = list.partition_point(|b| b.id.clock.value < block.id.clock.value);
        list.insert(pos, block);
    }

    pub fn get(&self, id: &BlockId) -> Option<&Block> {
        let list = self.blocks.get(&id.client)?;
        let idx = Self::find_index(list, id.clock.value)?;
        Some(&list[idx])
    }

    pub fn get_mut(&mut self, id: &BlockId) -> Option<&mut Block> {
        let list = self.blocks.get_mut(&id.client)?;
        let idx = Self::find_index(list, id.clock.value)?;
        Some(&mut list[idx])
    }

    pub fn state_vector(&self) -> StateVector {
        let mut sv = StateVector::new();
        for (client, blocks) in &self.blocks {
            if let Some(last) = blocks.last() {
                sv.update(*client, last.id.clock.value + last.len);
            }
        }
        sv
    }

    pub fn mark_deleted(&mut self, id: &BlockId) -> Option<&mut Block> {
        debug!("marking deleted block {:?}", id);
        let block = self.get_mut(id)?;
        if !block.is_deleted {
            block.is_deleted = true;
        }
        Some(block)
    }

    pub fn erase_content(&mut self, id: &BlockId) -> bool {
        debug!("erase content block {:?}", id);
        match self.get_mut(id) {
            Some(block) if block.is_deleted && !block.is_empty() => {
                block.clear_content_for_gc();
                true
            }
            _ => false,
        }
    }

    fn find_index(list: &[Block], clock: u64) -> Option<usize> {
        let result = list.partition_point(|b| b.id.clock.value <= clock);
        if result == 0 {
            return None;
        }
        let idx = result - 1;
        let b = &list[idx];
        if clock < b.id.clock.value + b.len {
            Some(idx)
        } else {
            None
        }
    }

    pub fn all_blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.values().flat_map(|v| v.iter())
    }

    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut map: HashMap<ClientId, Vec<Block>> = HashMap::new();
        for block in blocks {
            map.entry(block.id.client).or_default().push(block);
        }
        for list in map.values_mut() {
            list.sort_unstable_by_key(|b| b.id.clock.value);
        }
        StructStore { blocks: map }
    }
}
