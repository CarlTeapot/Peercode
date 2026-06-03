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

    pub fn merge(&mut self, other: &DeleteSet) {
        for (client, ranges) in &other.ranges {
            let list = self.ranges.entry(*client).or_default();
            for range in ranges {
                list.push(range.clone());
            }
            Self::compress(list);
        }
    }

    /// Remove every clock covered by `other`. Assumes both sets are compressed
    /// (sorted, disjoint), as the public API guarantees. Idempotent.
    pub fn subtract(&mut self, other: &DeleteSet) {
        for (client, holes) in &other.ranges {
            let result: Vec<DeleteRange> = match self.ranges.get(client) {
                Some(list) => {
                    let mut out = Vec::new();
                    for r in list {
                        Self::subtract_holes_from_range(r, holes, &mut out);
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
mod tests {
    use super::*;
    use crate::types::Clock;

    fn bid(client: u64, clock: u64) -> BlockId {
        BlockId::new(ClientId::new(client), Clock::new(clock))
    }

    fn ds(entries: &[(u64, u64, u64)]) -> DeleteSet {
        // (client, start, len)
        let mut d = DeleteSet::new();
        for &(c, start, len) in entries {
            d.add(bid(c, start), len);
        }
        d
    }

    fn covered(d: &DeleteSet, client: u64, lo: u64, hi: u64) -> Vec<u64> {
        (lo..hi).filter(|&k| d.contains(&bid(client, k))).collect()
    }

    #[test]
    fn subtract_full_range_removes_client() {
        let mut d = ds(&[(1, 0, 5)]);
        d.subtract(&ds(&[(1, 0, 5)]));
        assert!(d.is_empty());
    }

    #[test]
    fn subtract_middle_leaves_two_pieces() {
        let mut d = ds(&[(1, 0, 10)]);
        d.subtract(&ds(&[(1, 3, 3)])); // remove [3,6)
        assert_eq!(covered(&d, 1, 0, 10), vec![0, 1, 2, 6, 7, 8, 9]);
    }

    #[test]
    fn subtract_partial_overlap_trims_left_edge() {
        let mut d = ds(&[(1, 5, 5)]); // [5,10)
        d.subtract(&ds(&[(1, 0, 7)])); // remove [0,7)
        assert_eq!(covered(&d, 1, 0, 12), vec![7, 8, 9]);
    }

    #[test]
    fn subtract_unrelated_client_is_noop() {
        let mut d = ds(&[(1, 0, 5)]);
        d.subtract(&ds(&[(2, 0, 5)]));
        assert_eq!(covered(&d, 1, 0, 5), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn subtract_is_idempotent() {
        let mut d = ds(&[(1, 0, 10), (2, 0, 4)]);
        let confirmed = ds(&[(1, 2, 3)]);
        d.subtract(&confirmed);
        let after_first = covered(&d, 1, 0, 10);
        d.subtract(&confirmed);
        assert_eq!(covered(&d, 1, 0, 10), after_first);
        assert_eq!(covered(&d, 2, 0, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn subtract_spanning_multiple_ranges() {
        // two disjoint ranges; one hole spans across the gap
        let mut d = ds(&[(1, 0, 3), (1, 6, 3)]); // [0,3) and [6,9)
        d.subtract(&ds(&[(1, 2, 5)])); // remove [2,7)
        assert_eq!(covered(&d, 1, 0, 9), vec![0, 1, 7, 8]);
    }
}
