use serde_json::json;

use super::*;

fn error(message: &str, data: Option<Value>) -> RpcError {
    RpcError { code: Some(-32600), message: message.to_string(), data }
}

#[test]
fn a_finished_turn_and_a_replaced_turn_both_mean_the_turn_moved() {
    assert_eq!(classify_steer(&error("no active turn to steer", None)), Some(SteerRefusal::TurnMoved));
    assert_eq!(classify_steer(&error("expected active turn id a but found b", None)), Some(SteerRefusal::TurnMoved));
}

#[test]
fn a_non_steerable_turn_is_recognised_only_from_the_structured_data() {
    let data = json!({"codexErrorInfo": {"activeTurnNotSteerable": {"turnKind": "review"}}});
    assert_eq!(classify_steer(&error("cannot steer a review turn", Some(data))), Some(SteerRefusal::NotSteerable));
    assert_eq!(classify_steer(&error("cannot steer a review turn", None)), None);
}

#[test]
fn any_other_steer_error_is_not_classified() {
    assert_eq!(classify_steer(&error("direct app-server input is not allowed for multi-agent v2 sub-agents", None)), None);
}

#[test]
fn the_active_turn_is_the_newest_one_when_it_is_in_progress() {
    let resumed = json!({"thread": {"turns": []}, "initialTurnsPage": {"data": [{"id": "b", "status": "inProgress"}]}});
    assert_eq!(active_turn_id(&resumed), Some("b"));
}

#[test]
fn a_finished_newest_turn_or_no_page_means_no_active_turn_id() {
    let done = json!({"initialTurnsPage": {"data": [{"id": "a", "status": "completed"}]}});
    assert_eq!(active_turn_id(&done), None);
    assert_eq!(active_turn_id(&json!({"initialTurnsPage": {"data": []}})), None);
    assert_eq!(active_turn_id(&json!({"thread": {}})), None);
}

#[test]
fn a_lost_answer_is_the_only_handoff_failure_that_may_have_landed() {
    assert!(matches!(handoff(Err(CallError::Lost("x".into()))), Err(Outcome::MayNotHaveLanded(_))));
    assert!(matches!(handoff(Err(CallError::NotSent("x".into()))), Err(Outcome::Failed(_))));
    assert!(matches!(handoff(Err(CallError::Rejected(error("x", None)))), Err(Outcome::Failed(_))));
    assert!(handoff(Ok(json!({}))).is_ok());
}
