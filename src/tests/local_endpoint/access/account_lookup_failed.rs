use super::*;

#[test]
fn a_failed_lookup_stops_the_bind() {
    assert!(granted_sandbox_sids(|_| anyhow::bail!("lookup broke")).is_err());
}
