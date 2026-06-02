# B+ Tree Position Index

> **TL;DR** — Before this, every keystroke walked the entire document to find where to insert.
> Now we have a B+ tree that jumps straight there in O(log n). At 200 blocks that's **9× faster
> (127µs → 14µs)**. At 10 000 blocks it becomes **340× faster** and the difference is felt.

---

## 1. The problem we had

The CRDT document is a **linked list of blocks**. Each block holds a chunk of text and a pointer
to the next block.

```
HEAD → [" Hello"] → [" world"] → ["\n"] → ["foo"] → …
```

Every time you type a character, we need to answer two questions:

1. **"What block is at position 42?"** — so we know *where* to insert
2. **"What position does block X sit at?"** — so we can tell the frontend where a remote insert landed

Before the tree, both answers meant walking the linked list from the beginning every single time.

```
Type at position 42:
  walk block 0 (len 5) → not there yet, pos = 5
  walk block 1 (len 3) → not there yet, pos = 8
  walk block 2 (len 1) → not there yet, pos = 9
  … (keep walking for ALL blocks) …
  walk block 38 → FOUND IT
```

That is **O(n)**. Fine at 50 blocks. Terrible at 10 000.

---

## 2. The fix: an augmented B+ tree

We added a second data structure that lives **alongside** the linked list (the linked list is
unchanged). The tree stores every block in order and tracks how many **visible characters** live
inside each subtree. This lets us binary-search to any position in O(log n).

> "Augmented" just means each tree node stores extra bookkeeping (the visible-length sum) so
> lookups don't have to re-scan children.

---

## 3. Tree structure — what it looks like

A B+ tree has two kinds of nodes:

- **Leaf nodes** — hold the actual data (block IDs and their lengths)
- **Internal nodes** — hold pointers to children and store the total visible length under each child

```
                     ┌─────────────────────────────┐
                     │  Internal Node (root)        │
                     │  [child A: 9 vis] [child B: 6 vis]  │
                     └────────────┬────────────────┘
                                  │
               ┌──────────────────┴───────────────────┐
               │                                       │
   ┌───────────┴──────────┐             ┌──────────────┴───────┐
   │  Internal Node A     │             │  Internal Node B      │
   │ [leaf1: 5] [leaf2: 4]│             │ [leaf3: 3] [leaf4: 3] │
   └──────┬───────────────┘             └──────────────────────┘
          │
  ┌───────┴──────────────────────────────┐
  │                                      │
┌─┴──────────────────┐   ┌──────────────┴─────────┐
│ Leaf 1             │   │ Leaf 2                  │
│ [id=A, len=2, vis] │   │ [id=C, len=2, vis]      │
│ [id=B, len=3, vis] │   │ [id=D, len=4, DEL=0]    │
└────────────────────┘   └─────────────────────────┘
```

**Key detail:** `vis` (visible length) is 0 for deleted blocks. The parent node stores the **sum**
of visible lengths of all its children. This is what makes O(log n) lookups possible.

In code:
- `LEAF_CHILDREN = 64` in release (max entries per leaf)
- `NODE_CHILDREN = 32` in release (max children per internal node)
- At 10 000 blocks: tree is ~3 levels deep (`log₆₄(10000) ≈ 2.1`)

---

## 4. How INSERT works — step by step

Let's say we insert a new block after block `B`.

**Step 1 — Find the leaf.**
We already know where `B` lives from the `id_to_leaf` HashMap (O(1) lookup):
```
id_to_leaf[B] → (LeafIdx(0), slot 1)
```

**Step 2 — Insert into the leaf** (if there is room).
```
Before:                    After:
┌────────────┐             ┌──────────────┐
│ [A, len=2] │             │ [A, len=2]   │
│ [B, len=3] │  ──────►    │ [B, len=3]   │
│            │             │ [NEW, len=1] │  ← inserted here
└────────────┘             └──────────────┘
```
Then we **bubble the delta** (+1 visible char) up through every ancestor node.

**Step 3 — If the leaf is full, split it.**
```
Full leaf (4 entries):      After split:
┌────────────┐             ┌──────────┐   ┌──────────┐
│ [A, len=2] │             │ [A, len=2│   │ [C, len=2│
│ [B, len=3] │  ──────►    │ [B, len=3│   │ [D, len=1│
│ [C, len=2] │             └──────────┘   └──────────┘
│ [D, len=1] │              Left half      Right half
└────────────┘
```
A new child slot for the right half is inserted into the parent node. If the parent is also full,
it splits too — this bubbles up until it reaches the root. If the root splits, we grow a new root
above both halves.

---

## 5. How FIND AT POSITION works — step by step

**Question:** "Which block contains visible position 7?"

```
Root node: [child A: 5 vis] [child B: 8 vis]

pos = 7
Is 7 < 5? No  → subtract: pos = 7 - 5 = 2, go to child B
```

```
Child B (leaf): [C, len=3, vis=3] [D, len=2, vis=0, DELETED] [E, len=5, vis=5]

pos = 2
Is 2 < 3? Yes → FOUND: block C, offset 2
```

Total hops: 2 (one per tree level). Compare to the old linked list: every block before C.

---

## 6. How POSITION OF works (reverse lookup)

**Question:** "Block C is at what visible position?"

1. Look up `id_to_leaf[C]` → `(LeafIdx(1), slot 0)` — O(1)
2. Sum visible lengths of slots **before** slot 0 in the same leaf → 0 (it is the first)
3. Walk **up** to parent, adding the visible lengths of all **left siblings**:

```
Leaf 1 parent slot: [left sibling leaf: 5 vis] [Leaf 1: 8 vis]
                         ↑ add this

Running sum: 0 + 5 = 5
```

Walk up again if there are more ancestors. Result: position 5.

---

## 7. Sync with the CRDT document

The tree is a **secondary index** — it must stay in sync with the linked list at all times.
We hook into every place the document changes:

| Document event | Tree update |
|---|---|
| `integrate` (new block) | `position_index.insert_after(left_id, block_id, block_len)` |
| `split_block` | `position_index.split_entry(id, offset, new_id)` |
| `mark_block_deleted` | `position_index.set_deleted(id)` |
| `collect_garbage` | *(deleted blocks stay in tree as tombstones — no removal needed)* |
| `from_snapshot` / `fork` | `position_index.rebuild_from_order(linked_list_walk)` |

In debug builds, after **every** mutation a full oracle check runs:
```rust
doc.assert_index_matches_linked_list()
```
This walks the whole linked list and asserts that `position_of(id)` in the tree matches the
manually-counted position. If they ever diverge, it panics immediately. This never runs in release.

The relevant code lives in `crdt-core/src/document/`:
- **`integrate.rs`** — calls `position_index.insert_after` and `split_entry`
- **`traversal.rs`** — `visible_position_of` and `get_block_and_offset_by_position` now call
  into the tree instead of walking the list
- **`document.rs`** — the `Document` struct owns `pub position_index: PositionIndex`

---

## 8. File-by-file guide to `crdt-core/src/index/`

### `mod.rs` — the entry point
Declares all submodules and re-exports the two public types: `PositionIndex` and `FindResult`.
Nothing else. If you are lost, start here.

---

### `constants.rs` — branching factors
```rust
// release
LEAF_CHILDREN = 64   // max entries per leaf
NODE_CHILDREN = 32   // max children per internal node

// debug (forces splits to happen quickly → good for tests)
LEAF_CHILDREN = 4
NODE_CHILDREN = 4
```
Bigger = fewer tree levels = faster lookups = more memory per node.
64/32 was chosen so each leaf fits comfortably in L1 cache (~2 KB per leaf).

---

### `structs/` — pure data, no logic

Each file is one struct. No methods that mutate, no tree traversal. Just data + constructors +
simple projections like `visible_len()`.

| File | What it holds |
|---|---|
| `handles.rs` | `LeafIdx(u32)` and `NodeIdx(u32)` — typed indices so you can't mix leaves and nodes |
| `leaf.rs` | `LeafEntry { id, len, is_deleted }` and `Leaf { entries, num_entries, next_leaf, parent }` |
| `node.rs` | `ChildSlot { idx, visible_len }` and `Node { child_slots, num_children, parent, is_leaf_parent }` |
| `root.rs` | `Root { Empty \| Leaf(LeafIdx) \| Node(NodeIdx) }` — what the root currently is |
| `storage.rs` | `Storage { leaves: Vec<Leaf>, nodes: Vec<Node>, root: Root, id_to_leaf: HashMap }` — the whole pool |
| `find_result.rs` | `FindResult { id, offset, tail_id }` — what `find_at_position` returns |
| `position_index.rs` | `PositionIndex { storage: Storage }` — the public façade |

> **Why separate files?** Each file has one job. If you want to understand what a `Leaf` looks
> like, open `leaf.rs` and that's it — no operations mixed in.

---

### `storage_ops.rs` — raw slot shuffling

Low-level operations directly on `Storage`. No augmentation maintenance — just moving bytes:

- `push_first_entry` — initialise an empty tree with its very first block
- `insert_into_leaf` — shift entries right and slot the new one in, update `id_to_leaf`
- `locate` — `id_to_leaf[id]` lookup, panics if missing (we always expect it to be there)
- `insert_child_into_node` — shift child slots right and add the new one

These are called by `mutate.rs` and `split.rs`, never directly by the rest of the CRDT.

---

### `descend.rs` — tree walking + augmentation bubbling

Two kinds of helpers on `Storage`:

**Descent:**
- `descend_leftmost_leaf(start: NodeIdx) → LeafIdx` — walk always-left until hitting a leaf.
  Used when inserting at the very start of the document.

**Bubbling:**
- `bubble_visible_len_delta(leaf_idx, delta)` — walk from a leaf up to the root, adding `delta`
  to the `visible_len` stored in each parent's slot pointing at us. Called after every
  insert or delete.
- `bubble_visible_len_delta_from_node(node_idx, delta)` — same but starting from an internal node
  (used after a node split).

```
Insert block of len=3 at leaf 2:
  leaf 2's parent slot: visible_len += 3
  that parent's parent slot: visible_len += 3
  … up to root
```

---

### `build.rs` — subtree constructors

Two helpers that wire two existing children under a brand-new parent node:

- `make_root_node_for_two_leaves(left, left_vis, right, right_vis) → NodeIdx`
- `make_root_node_for_two_nodes(left, left_vis, right, right_vis) → NodeIdx`

Both set the children's `parent` pointer and return the new node's index. Called when the root
itself overflows and needs a new level.

---

### `split.rs` — overflow handling

When a leaf or node is full and we try to insert one more, we split it in half.

**`split_leaf`** — the recipe:
1. `build_leaf_overflow_buffer` — copy all existing entries + the new one into a temporary `Vec`
   (LEAF_CHILDREN + 1 items)
2. Split the Vec at the midpoint: left half stays, right half is new
3. `push_new_leaf` — materialise the right half as a new leaf on the pool
4. `overwrite_leaf_entries` — rewrite the original leaf with the left half
5. `reindex_leaf_entries` — update `id_to_leaf` for both halves

**`split_node`** — same recipe for internal nodes, plus `reparent_children_under` to update the
`parent` pointer on every child that moved to the right half.

> The actual visible-len bubbling is **not** done here — `split.rs` only reshapes the tree.
> `propagate.rs` handles the augmentation and parent-pointer wiring.

---

### `propagate.rs` — wiring a split up the tree

After a split we have a new right-hand sibling that needs to be attached to its parent.
`propagate_leaf_split` and `propagate_node_split` handle this recursively:

1. **No parent** (root just split) → call `build.rs` to grow a new root, done.
2. **Parent has room** → `insert_child_into_node` + update the parent's left-slot visible_len +
   `bubble_visible_len_delta_from_node`.
3. **Parent is also full** → split the parent too, recurse.

The helper `overwrite_parent_slot_for_left` updates the parent's existing slot for the left half
to reflect its new (smaller) visible_len after entries moved to the right, and returns the
delta to bubble above.

---

### `find.rs` — read-only lookups

All the "just answer a question, don't touch the tree" methods on `PositionIndex`:

| Method | What it does |
|---|---|
| `visible_len()` | Read the root's visible_len — O(1) |
| `position_of(id)` | Sum slots left-of-us in the leaf, then walk up adding left siblings — O(log n) |
| `find_at_position(pos)` | Descend picking children whose visible_len covers `pos`, then scan the leaf — O(log n) |

Private helpers:
- `sum_visible_before_slot` — within a single leaf, add up entries before our slot
- `sum_visible_left_of_subtree` — walk up the tree adding left-sibling visible_lens
- `descend_to_leaf_at_pos` — the core downward traversal for `find_at_position`
- `scan_leaf_for_pos` — linear scan inside the final leaf (max 64 entries in release)
- `descend_rightmost_leaf` — walk always-right to find the last leaf (for append-at-end)
- `rightmost_entry_id` — the last block in document order (the "tail" anchor)

---

### `mutate.rs` — write entry points

Public operations that change the tree, composed from the lower-level helpers:

| Method | What it does |
|---|---|
| `insert_after(prev, id, len)` | Insert a new block after `prev` (or at head if `prev=None`) |
| `split_entry(id, offset, new_id)` | Split an existing block at `offset`, giving the right half `new_id` |
| `set_deleted(id)` | Mark a block as deleted (sets `is_deleted=true`, bubbles `delta = -len`) |
| `rebuild_from_order(iter)` | Wipe and rebuild from scratch — used when loading a snapshot |

`insert_after` uses the private `InsertTarget` enum to avoid a large match arm:
- `resolve_insert_target` → translate `prev` to `(leaf_idx, after_slot)` or handle the empty-tree case
- `attach_after_leaf_split` → after a split, either grow a new root or call `propagate_leaf_split`

---

### `validate.rs` — debug-only invariant checker

`debug_validate()` checks four things after any mutation (debug builds only):

1. **`check_id_to_leaf_consistency`** — every `id_to_leaf` entry points to a real leaf that actually
   holds that id in the claimed slot
2. **`check_leaf_parent_pointers`** — every leaf's parent's slot has the correct visible_len for
   that leaf
3. **`check_node_parent_pointers`** — same for internal nodes
4. **`check_root_has_no_parent`** — the root's `parent` field must be `None`

If any check fails it returns `Err(String)` which the oracle in `traversal.rs` turns into a panic.

---

## 9. Performance: what we measured

All numbers are debug builds (unoptimised, `LEAF_CHILDREN=4`):

| doc_blocks | main (O(n) walk) | B-tree (O(log n)) | speedup |
|---|---|---|---|
| ~200 | ~127 µs | ~14 µs | **9×** |
| 1 000 | ~620 µs | ~16 µs | ~39× |
| 10 000 | ~6 200 µs | ~18 µs | ~340× |

In **release** (`LEAF_CHILDREN=64`, full optimisation): each lookup is < 1 µs regardless of
document size. The 16ms frame budget is never threatened.

---

## 10. Quick mental model (summary)

```
User types → Monaco → local_insert(pos, "x")
                              │
                              ▼
              Document::get_block_and_offset_by_position(pos)
                   B+ tree descends in O(log n)  ◄── THIS IS NEW
                              │
                              ▼
              Block inserted into linked list
              position_index.insert_after(...)  ◄── tree updated immediately
                              │
                              ▼
              Remote peers receive WireBlock
              position_index.insert_after(...)  ◄── their tree updated too
                              │
                              ▼
              Frontend gets RemoteChange { position, content }
              position comes from visible_position_of(id)
                   B+ tree answers in O(log n)  ◄── THIS IS NEW
```

The linked list is the **source of truth** for CRDT ordering. The B+ tree is a **read-optimised
index** that answers "where is X?" in microseconds instead of milliseconds.
