---
description: B+ tree position index internals for the crdt-core CRDT
globs:
  - crdt-core/**
alwaysApply: false
---

# crdt-core position index

`crdt-core/src/index/` is an augmented B+ tree that gives O(log n) conversion
between a `BlockId` and its visible-text character position. It is a *secondary,
derived* structure layered over the CRDT linked list (the linked list, traversed
via `block.right()`, remains the source of truth for order).

Key facts before touching `index/` or any `Document` mutation:

- The index must move in lockstep with the linked list. Every mutation that
  changes block order, length, or visibility mirrors itself into `position_index`
  (`insert_after`, `split_entry`, `set_deleted`, `rebuild_from_order`).
- In debug builds, `assert_index_matches_linked_list` (`document/debug.rs`) and
  `index/validate.rs` panic on any drift. These are `#[cfg(debug_assertions)]`.
- Module split: `index/structs/` is data + constructors only; sibling files
  (`mutate`, `find`, `split`, `propagate`, `descend`, `build`, `storage_ops`)
  hold operations. Branching factors in `index/constants.rs` differ between
  debug (tiny, forces splits in tests) and release (wide).

Full walkthrough with diagrams and a file-by-file guide:

@../../docs/b-tree-optimisation.md
