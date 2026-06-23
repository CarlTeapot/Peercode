use crate::types::BlockId;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,

    // Original neighbors.
    pub origin_left: Option<BlockId>,
    pub origin_right: Option<BlockId>,

    // Current neighbors.
    left: Option<BlockId>,
    right: Option<BlockId>,

    content: String,

    pub is_deleted: bool,

    pub len: u64,
}

impl Block {
    pub fn new(
        id: BlockId,
        origin_left: Option<BlockId>,
        origin_right: Option<BlockId>,
        content: String,
    ) -> Self {
        let len = content.chars().count() as u64;

        Block {
            id,
            origin_left,
            origin_right,
            left: origin_left,
            right: origin_right,
            content,
            is_deleted: false,
            len,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: BlockId,
        origin_left: Option<BlockId>,
        origin_right: Option<BlockId>,
        left: Option<BlockId>,
        right: Option<BlockId>,
        content: String,
        is_deleted: bool,
        len: u64,
    ) -> Self {
        Block {
            id,
            origin_left,
            origin_right,
            left,
            right,
            content,
            is_deleted,
            len,
        }
    }

    pub fn left(&self) -> Option<BlockId> {
        self.left
    }

    pub fn right(&self) -> Option<BlockId> {
        self.right
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, content: String) {
        self.len = content.chars().count() as u64;
        self.content = content;
    }

    pub fn set_left(&mut self, id: Option<BlockId>) {
        self.left = id;
    }

    pub fn set_right(&mut self, id: Option<BlockId>) {
        self.right = id;
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn clear_content_for_gc(&mut self) {
        self.content = String::new();
    }

    // empty for now , will implement later when we have the basic structure in place
    //pub fn split(self, offset: u64) -> (Block, Block) {}

    //pub fn squash(self, other: Block) -> Result<Block, (Block, Block)> {}
}
