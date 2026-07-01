use super::*;
use crate::types::Clock;

fn bid(client: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId::new(client), Clock::new(clock))
}

fn ds(entries: &[(u64, u64, u64)]) -> DeleteSet {
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
    d.subtract(&ds(&[(1, 3, 3)]));
    assert_eq!(covered(&d, 1, 0, 10), vec![0, 1, 2, 6, 7, 8, 9]);
}

#[test]
fn subtract_partial_overlap_trims_left_edge() {
    let mut d = ds(&[(1, 5, 5)]);
    d.subtract(&ds(&[(1, 0, 7)]));
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
    let mut d = ds(&[(1, 0, 3), (1, 6, 3)]);
    d.subtract(&ds(&[(1, 2, 5)]));
    assert_eq!(covered(&d, 1, 0, 9), vec![0, 1, 7, 8]);
}
