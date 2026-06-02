use crate::index::structs::handles::{LeafIdx, NodeIdx};

#[derive(Debug, Clone, Copy)]
pub(in crate::index) enum Root {
    Empty,
    Leaf(LeafIdx),
    Node(NodeIdx),
}
