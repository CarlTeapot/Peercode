#![cfg(debug_assertions)]
use crate::index::structs::position_index::PositionIndex;
use crate::index::structs::root::Root;

impl PositionIndex {
    pub fn debug_validate(&self) -> Result<(), String> {
        self.check_id_to_leaf_consistency()?;
        self.check_leaf_parent_pointers()?;
        self.check_node_parent_pointers()?;
        self.check_root_has_no_parent()
    }

    fn check_id_to_leaf_consistency(&self) -> Result<(), String> {
        for (id, (leaf_idx, slot)) in self.storage.id_to_leaf.iter() {
            let leaf = self
                .storage
                .leaves
                .get(leaf_idx.0 as usize)
                .ok_or_else(|| format!("id_to_leaf points to missing leaf {:?}", leaf_idx))?;
            let entry = leaf
                .entries
                .get(*slot as usize)
                .and_then(|e| e.as_ref())
                .ok_or_else(|| format!("slot {} empty for id {:?}", slot, id))?;
            if entry.id != *id {
                return Err(format!(
                    "id mismatch at {:?}/{}: stored {:?}, mapped {:?}",
                    leaf_idx, slot, entry.id, id
                ));
            }
        }
        Ok(())
    }

    fn check_leaf_parent_pointers(&self) -> Result<(), String> {
        for (li, leaf) in self.storage.leaves.iter().enumerate() {
            let Some(parent) = leaf.parent else { continue };
            let node = &self.storage.nodes[parent.0 as usize];
            let mut found = false;
            for slot in node.child_slots.iter().take(node.num_children as usize) {
                let s = slot.expect("populated slot");
                if s.idx as usize != li {
                    continue;
                }
                if s.visible_len != leaf.visible_len() {
                    return Err(format!(
                        "leaf {} visible_len {} disagrees with parent slot {}",
                        li,
                        leaf.visible_len(),
                        s.visible_len
                    ));
                }
                found = true;
                break;
            }
            if !found {
                return Err(format!(
                    "leaf {} has parent {:?} but no slot points to it",
                    li, parent
                ));
            }
        }
        Ok(())
    }

    fn check_node_parent_pointers(&self) -> Result<(), String> {
        for (ni, node) in self.storage.nodes.iter().enumerate() {
            let Some(parent) = node.parent else { continue };
            let parent_node = &self.storage.nodes[parent.0 as usize];
            let mut found = false;
            for slot in parent_node
                .child_slots
                .iter()
                .take(parent_node.num_children as usize)
            {
                let s = slot.expect("populated slot");
                if s.idx as usize != ni {
                    continue;
                }
                if s.visible_len != node.visible_len() {
                    return Err(format!(
                        "node {} visible_len {} disagrees with parent slot {}",
                        ni,
                        node.visible_len(),
                        s.visible_len
                    ));
                }
                found = true;
                break;
            }
            if !found {
                return Err(format!(
                    "node {} has parent {:?} but no slot points to it",
                    ni, parent
                ));
            }
        }
        Ok(())
    }

    fn check_root_has_no_parent(&self) -> Result<(), String> {
        match self.storage.root {
            Root::Empty => Ok(()),
            Root::Leaf(l) => {
                if self.storage.leaves[l.0 as usize].parent.is_some() {
                    Err("root leaf has a parent".to_string())
                } else {
                    Ok(())
                }
            }
            Root::Node(n) => {
                if self.storage.nodes[n.0 as usize].parent.is_some() {
                    Err("root node has a parent".to_string())
                } else {
                    Ok(())
                }
            }
        }
    }
}
