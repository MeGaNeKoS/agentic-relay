use super::*;
use std::collections::HashMap;

fn chain_of(parents: &[(u32, u32)], starts: &[(u32, u64)], pid: u32) -> Result<Vec<u32>> {
    let parents: HashMap<u32, u32> = parents.iter().copied().collect();
    let starts: HashMap<u32, u64> = starts.iter().copied().collect();
    build_chain(pid, |p| parents.get(&p).copied(), |p| starts.get(&p).copied()).map(|c| c.iter().map(|a| a.pid).collect())
}

#[test]
fn walks_older_parents_up_to_the_root() {
    let chain = chain_of(&[(3, 2), (2, 1)], &[(3, 30), (2, 20), (1, 10)], 3).unwrap();
    assert_eq!(chain, vec![3, 2, 1]);
}

#[test]
fn stops_at_a_parent_that_started_after_its_child() {
    let chain = chain_of(&[(3, 2), (2, 1)], &[(3, 30), (2, 20), (1, 25)], 3).unwrap();
    assert_eq!(chain, vec![3, 2]);
}

#[test]
fn stops_at_a_parent_that_started_at_the_same_instant() {
    let chain = chain_of(&[(3, 2)], &[(3, 30), (2, 30)], 3).unwrap();
    assert_eq!(chain, vec![3]);
}

#[test]
fn stops_at_a_parent_that_is_gone() {
    let chain = chain_of(&[(3, 2), (2, 1)], &[(3, 30), (1, 10)], 3).unwrap();
    assert_eq!(chain, vec![3]);
}

#[test]
fn a_cycle_cannot_loop_forever() {
    let chain = chain_of(&[(2, 1), (1, 2)], &[(2, 20), (1, 10)], 2).unwrap();
    assert_eq!(chain, vec![2, 1]);
}

#[test]
fn an_unreadable_start_for_the_process_itself_is_an_error() {
    assert!(chain_of(&[], &[], 9).is_err());
}
