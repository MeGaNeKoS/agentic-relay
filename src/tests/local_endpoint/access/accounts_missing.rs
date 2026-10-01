use super::*;

#[test]
fn a_missing_sandbox_account_is_skipped_and_the_other_is_still_granted() {
    let sids = granted_sandbox_sids(|name| Ok((name == "CodexSandboxOnline").then(|| "S-1-5-21-9".to_string()))).unwrap();
    assert_eq!(sids, ["S-1-5-21-9"]);
}

#[test]
fn with_no_sandbox_accounts_nothing_is_granted() {
    assert!(granted_sandbox_sids(|_| Ok(None)).unwrap().is_empty());
}
