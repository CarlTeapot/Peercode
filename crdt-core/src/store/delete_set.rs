use crate::types::{BlockId, ClientId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct DeleteRange {
    pub start: u64,
    pub len: u64,
}

impl DeleteRange {
    #[inline]
    pub fn end(&self) -> u64 {
        self.start + self.len
    }

    fn overlaps_or_adjacent(&self, other: &DeleteRange) -> bool {
        self.start <= other.end() && other.start <= self.end()
    }

    fn merge_with(&self, other: &DeleteRange) -> DeleteRange {
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        DeleteRange {
            start,
            len: end - start,
        }
    }

    pub fn contains(&self, clock: u64) -> bool {
        clock >= self.start && clock < self.end()
    }
}

#[derive(Debug, Clone, Default, PartialEq, bitcode::Encode, bitcode::Decode)]
pub struct DeleteSet {
    ranges: HashMap<ClientId, Vec<DeleteRange>>,
}

impl DeleteSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, id: BlockId, len: u64) {
        if len == 0 {
            return;
        }
        let new_range = DeleteRange {
            start: id.clock.value,
            len,
        };
        let list = self.ranges.entry(id.client).or_default();
        list.push(new_range);
        Self::compress(list);
    }

    pub fn contains(&self, id: &BlockId) -> bool {
        match self.ranges.get(&id.client) {
            Some(list) => {
                let idx = list.partition_point(|r| r.start <= id.clock.value);
                if idx > 0 {
                    list[idx - 1].contains(id.clock.value)
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub fn covers(&self, id: &BlockId, len: u64) -> bool {
        if len == 0 {
            return true;
        }
        match self.ranges.get(&id.client) {
            Some(list) => {
                let idx = list.partition_point(|r| r.start <= id.clock.value);
                if idx == 0 {
                    return false;
                }
                let range = &list[idx - 1];
                id.clock.value >= range.start && id.clock.value + len <= range.end()
            }
            None => false,
        }
    }

    pub fn merge(&mut self, other: &DeleteSet) {
        for (client, ranges) in &other.ranges {
            let list = self.ranges.entry(*client).or_default();
            for range in ranges {
                list.push(range.clone());
            }
            Self::compress(list);
        }
    }

    /// Remove every clock covered by `holes`. Assumes both sets are compressed
    /// (sorted, disjoint), as the public API guarantees. Idempotent.
    pub fn subtract(&mut self, holes: &DeleteSet) {
        for (client, client_holes) in &holes.ranges {
            let result: Vec<DeleteRange> = match self.ranges.get(client) {
                Some(list) => {
                    let mut out = Vec::new();
                    for r in list {
                        Self::subtract_holes_from_range(r, client_holes, &mut out);
                    }
                    out
                }
                None => continue,
            };
            if result.is_empty() {
                self.ranges.remove(client);
            } else {
                self.ranges.insert(*client, result);
            }
        }
    }

    pub fn subtract_prefix(&mut self, client: ClientId, floor: u64) {
        let Some(list) = self.ranges.get(&client) else {
            return;
        };
        let mut out = Vec::new();
        for range in list {
            if range.end() <= floor {
                continue;
            }
            if range.start < floor {
                out.push(DeleteRange {
                    start: floor,
                    len: range.end() - floor,
                });
            } else {
                out.push(range.clone());
            }
        }
        if out.is_empty() {
            self.ranges.remove(&client);
        } else {
            self.ranges.insert(client, out);
        }
    }

    /// Emit the portions of `r` not covered by `holes` (sorted, disjoint) into `out`.
    fn subtract_holes_from_range(
        r: &DeleteRange,
        holes: &[DeleteRange],
        out: &mut Vec<DeleteRange>,
    ) {
        let mut cur = r.start;
        let end = r.end();
        for hole in holes {
            if hole.end() <= cur {
                continue;
            }
            if hole.start >= end {
                break;
            }
            if hole.start > cur {
                out.push(DeleteRange {
                    start: cur,
                    len: hole.start - cur,
                });
            }
            cur = cur.max(hole.end());
            if cur >= end {
                return;
            }
        }
        if cur < end {
            out.push(DeleteRange {
                start: cur,
                len: end - cur,
            });
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ClientId, &DeleteRange)> {
        self.ranges
            .iter()
            .flat_map(|(client, ranges)| ranges.iter().map(move |r| (client, r)))
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.values().all(|v| v.is_empty())
    }

    fn compress(list: &mut Vec<DeleteRange>) {
        if list.len() <= 1 {
            return;
        }
        list.sort_unstable_by_key(|r| r.start);

        let mut merged: Vec<DeleteRange> = Vec::with_capacity(list.len());
        for range in list.drain(..) {
            match merged.last_mut() {
                Some(last) if last.overlaps_or_adjacent(&range) => {
                    *last = last.merge_with(&range);
                }
                _ => merged.push(range),
            }
        }
        *list = merged;
    }
}

#[cfg(test)]
mod tests;
