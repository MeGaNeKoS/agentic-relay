use super::*;

#[test]
fn both_sandbox_accounts_are_looked_up_by_name() {
    let sids = granted_sandbox_sids(|name| Ok(Some(format!("sid-of-{name}")))).unwrap();
    assert_eq!(sids, ["sid-of-CodexSandboxOffline", "sid-of-CodexSandboxOnline"]);
}
