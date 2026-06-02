#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::index) struct LeafIdx(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::index) struct NodeIdx(pub u32);
