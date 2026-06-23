#[cfg(not(debug_assertions))]
pub(super) const LEAF_CHILDREN: usize = 64;
#[cfg(not(debug_assertions))]
pub(super) const NODE_CHILDREN: usize = 32;

#[cfg(debug_assertions)]
pub(super) const LEAF_CHILDREN: usize = 4;
#[cfg(debug_assertions)]
pub(super) const NODE_CHILDREN: usize = 4;
