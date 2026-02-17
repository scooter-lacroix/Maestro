use maestro_core::engine::{SessionStateError, ThreadSession, ThreadState};

#[test]
fn default_thread_starts_in_processing() {
    let session = ThreadSession::new();
    assert_eq!(session.state(), &ThreadState::Processing);
}

#[test]
fn transition_to_awaiting_approval_sets_request_id() {
    let mut session = ThreadSession::new();

    session
        .transition_to_awaiting_approval("req-123")
        .expect("transition should succeed");

    assert_eq!(session.state(), &ThreadState::AwaitingApproval);
    assert_eq!(session.pending_approval_request_id(), Some("req-123"));
}

#[test]
fn duplicate_awaiting_approval_transition_is_rejected_without_overwriting_request_id() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_approval("req-123")
        .expect("initial transition should succeed");

    let err = session
        .transition_to_awaiting_approval("req-456")
        .expect_err("duplicate pending approval transition should fail");

    match err {
        SessionStateError::InvalidState { expected, got } => {
            assert_eq!(expected, ThreadState::Processing);
            assert_eq!(got, ThreadState::AwaitingApproval);
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(session.state(), &ThreadState::AwaitingApproval);
    assert_eq!(session.pending_approval_request_id(), Some("req-123"));
}

#[test]
fn matching_approval_request_id_transitions_back_to_processing() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_approval("req-123")
        .expect("transition should succeed");

    session
        .submit_approval_decision("req-123", true)
        .expect("approval should succeed");

    assert_eq!(session.state(), &ThreadState::Processing);
    assert_eq!(session.pending_approval_request_id(), None);
}

#[test]
fn mismatched_approval_request_id_returns_explicit_error() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_approval("req-expected")
        .expect("transition should succeed");

    let err = session
        .submit_approval_decision("req-got", true)
        .expect_err("approval should fail on mismatched request id");

    match err {
        SessionStateError::ApprovalRequestIdMismatch { expected, got } => {
            assert_eq!(expected, "req-expected");
            assert_eq!(got, "req-got");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn approval_when_not_awaiting_approval_returns_state_error() {
    let mut session = ThreadSession::new();

    let err = session
        .submit_approval_decision("req-123", true)
        .expect_err("approval should fail when session is not awaiting approval");

    match err {
        SessionStateError::InvalidState { expected, got } => {
            assert_eq!(expected, ThreadState::AwaitingApproval);
            assert_eq!(got, ThreadState::Processing);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn second_approval_submission_fails_after_successful_first_submission() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_approval("req-123")
        .expect("transition should succeed");
    session
        .submit_approval_decision("req-123", true)
        .expect("approval should succeed");

    let err = session
        .submit_approval_decision("req-123", true)
        .expect_err("second approval should fail due to state mismatch");

    match err {
        SessionStateError::InvalidState { expected, got } => {
            assert_eq!(expected, ThreadState::AwaitingApproval);
            assert_eq!(got, ThreadState::Processing);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn awaiting_approval_with_pending_request_cannot_transition_to_awaiting_auth() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_approval("req-123")
        .expect("transition should succeed");

    let err = session
        .transition_to_awaiting_auth()
        .expect_err("transition should fail while approval is pending");

    match err {
        SessionStateError::InvalidState { expected, got } => {
            assert_eq!(expected, ThreadState::Processing);
            assert_eq!(got, ThreadState::AwaitingApproval);
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(session.state(), &ThreadState::AwaitingApproval);
    assert_eq!(session.pending_approval_request_id(), Some("req-123"));
}

#[test]
fn awaiting_approval_with_pending_request_cannot_mark_completed() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_approval("req-123")
        .expect("transition should succeed");

    let err = session
        .mark_completed()
        .expect_err("completion should fail while approval is pending");

    match err {
        SessionStateError::InvalidState { expected, got } => {
            assert_eq!(expected, ThreadState::Processing);
            assert_eq!(got, ThreadState::AwaitingApproval);
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(session.state(), &ThreadState::AwaitingApproval);
    assert_eq!(session.pending_approval_request_id(), Some("req-123"));
}

#[test]
fn awaiting_auth_completed_and_failed_states_are_representable() {
    let mut session = ThreadSession::new();
    session
        .transition_to_awaiting_auth()
        .expect("awaiting auth transition should succeed");
    assert_eq!(session.state(), &ThreadState::AwaitingAuth);
    session
        .mark_completed()
        .expect("completed transition should succeed");
    assert_eq!(session.state(), &ThreadState::Completed);
    session
        .mark_failed()
        .expect("failed transition should succeed");
    assert_eq!(session.state(), &ThreadState::Failed);
}
