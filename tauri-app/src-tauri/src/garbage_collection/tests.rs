use super::*;
use crdt_core::types::{BlockId, Clock};

fn sv(entries: &[(u64, u64)]) -> StateVector {
    StateVector::from_entries(
        entries
            .iter()
            .map(|&(c, n)| (ClientId::new(c), n))
            .collect(),
    )
}

fn ds(entries: &[(u64, u64, u64)]) -> DeleteSet {
    let mut d = DeleteSet::new();
    for &(c, start, len) in entries {
        d.add(BlockId::new(ClientId::new(c), Clock::new(start)), len);
    }
    d
}

#[test]
fn compactable_deletes_exist_when_range_starts_below_floor() {
    assert!(has_compactable_deletes(&ds(&[(1, 0, 10)]), &sv(&[(1, 6)])));
}

#[test]
fn compactable_deletes_do_not_exist_when_floor_is_zero() {
    assert!(!has_compactable_deletes(&ds(&[(1, 0, 5)]), &sv(&[])));
}

#[test]
fn compactable_deletes_do_not_exist_when_range_starts_at_floor() {
    assert!(!has_compactable_deletes(&ds(&[(1, 6, 3)]), &sv(&[(1, 6)])));
}

#[test]
fn min_sv_takes_per_client_minimum_treating_absent_as_zero() {
    let peer_svs = HashMap::from([
        (ClientId::new(7), sv(&[(1, 5), (2, 9)])),
        (ClientId::new(8), sv(&[(1, 8)])),
    ]);
    let own = sv(&[(1, 10), (2, 9)]);
    let min = compute_min_sv(&peer_svs, &own);
    assert_eq!(min.get(&ClientId::new(1)), 5);
    assert_eq!(min.get(&ClientId::new(2)), 0);
}

#[test]
fn stale_report_after_left_does_not_resurrect_peer() {
    let peer = ClientId::new(7);
    let mut peers = HashMap::new();
    apply_event(&mut peers, GcEvent::Joined(peer));
    apply_event(
        &mut peers,
        GcEvent::PeerSvReport {
            client: peer,
            sv: sv(&[(1, 5)]),
        },
    );
    apply_event(&mut peers, GcEvent::Left(peer));
    apply_event(
        &mut peers,
        GcEvent::PeerSvReport {
            client: peer,
            sv: sv(&[(1, 5)]),
        },
    );
    assert!(
        peers.is_empty(),
        "a report delivered after `left` must not re-add the peer"
    );
}

#[test]
fn reports_merge_per_client_max_into_joined_entry() {
    let peer = ClientId::new(7);
    let mut peers = HashMap::new();
    apply_event(&mut peers, GcEvent::Joined(peer));
    apply_event(
        &mut peers,
        GcEvent::PeerSvReport {
            client: peer,
            sv: sv(&[(1, 5)]),
        },
    );
    apply_event(
        &mut peers,
        GcEvent::PeerSvReport {
            client: peer,
            sv: sv(&[(1, 3), (2, 4)]),
        },
    );
    let entry = &peers[&peer];
    assert_eq!(entry.get(&ClientId::new(1)), 5);
    assert_eq!(entry.get(&ClientId::new(2)), 4);
}

#[test]
fn min_sv_equals_own_when_no_peers() {
    let own = sv(&[(1, 4), (2, 7)]);
    let min = compute_min_sv(&HashMap::new(), &own);
    assert_eq!(min.get(&ClientId::new(1)), 4);
    assert_eq!(min.get(&ClientId::new(2)), 7);
}

#[test]
fn document_replaced_clears_reported_svs_but_keeps_membership() {
    let mut peers: HashMap<ClientId, StateVector> = HashMap::new();
    let peer = ClientId::new(7);

    apply_event(&mut peers, GcEvent::Joined(peer));
    let mut sv = StateVector::new();
    sv.update(ClientId::new(1), 42);
    apply_event(&mut peers, GcEvent::PeerSvReport { client: peer, sv });

    apply_event(&mut peers, GcEvent::DocumentReplaced);

    assert!(peers.contains_key(&peer));

    assert_eq!(peers.get(&peer).unwrap().get(&ClientId::new(1)), 0);
}
