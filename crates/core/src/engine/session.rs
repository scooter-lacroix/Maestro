use thiserror::Error;

use crate::engine::state::ThreadState;

#[derive(Debug, Clone)]
pub struct ThreadSession {
    state: ThreadState,
    pending_approval_request_id: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionStateError {
    #[error("invalid state transition: expected {expected:?}, got {got:?}")]
    InvalidState { expected: ThreadState, got: ThreadState },
    #[error("missing pending approval request id while in AwaitingApproval state")]
    MissingApprovalRequestId,
    #[error("approval request id mismatch: expected '{expected}', got '{got}'")]
    ApprovalRequestIdMismatch { expected: String, got: String },
}

impl Default for ThreadSession {
    fn default() -> Self {
        Self {
            state: ThreadState::Processing,
            pending_approval_request_id: None,
        }
    }
}

impl ThreadSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &ThreadState {
        &self.state
    }

    pub fn pending_approval_request_id(&self) -> Option<&str> {
        self.pending_approval_request_id.as_deref()
    }

    fn has_pending_approval_context(&self) -> bool {
        self.state == ThreadState::AwaitingApproval && self.pending_approval_request_id.is_some()
    }

    pub fn transition_to_awaiting_approval(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<(), SessionStateError> {
        if self.has_pending_approval_context() {
            return Err(SessionStateError::InvalidState {
                expected: ThreadState::Processing,
                got: self.state.clone(),
            });
        }
        self.state = ThreadState::AwaitingApproval;
        self.pending_approval_request_id = Some(request_id.into());
        Ok(())
    }

    pub fn submit_approval_decision(
        &mut self,
        request_id: impl AsRef<str>,
        approved: bool,
    ) -> Result<(), SessionStateError> {
        if self.state != ThreadState::AwaitingApproval {
            return Err(SessionStateError::InvalidState {
                expected: ThreadState::AwaitingApproval,
                got: self.state.clone(),
            });
        }

        let expected = self
            .pending_approval_request_id
            .as_ref()
            .ok_or(SessionStateError::MissingApprovalRequestId)?;
        let got = request_id.as_ref();

        if expected != got {
            return Err(SessionStateError::ApprovalRequestIdMismatch {
                expected: expected.clone(),
                got: got.to_string(),
            });
        }

        self.pending_approval_request_id = None;
        self.state = if approved {
            ThreadState::Processing
        } else {
            ThreadState::Failed
        };

        Ok(())
    }

    pub fn transition_to_awaiting_auth(&mut self) -> Result<(), SessionStateError> {
        if self.has_pending_approval_context() {
            return Err(SessionStateError::InvalidState {
                expected: ThreadState::Processing,
                got: self.state.clone(),
            });
        }
        self.pending_approval_request_id = None;
        self.state = ThreadState::AwaitingAuth;
        Ok(())
    }

    pub fn mark_completed(&mut self) -> Result<(), SessionStateError> {
        if self.has_pending_approval_context() {
            return Err(SessionStateError::InvalidState {
                expected: ThreadState::Processing,
                got: self.state.clone(),
            });
        }
        self.pending_approval_request_id = None;
        self.state = ThreadState::Completed;
        Ok(())
    }

    pub fn mark_failed(&mut self) -> Result<(), SessionStateError> {
        self.pending_approval_request_id = None;
        self.state = ThreadState::Failed;
        Ok(())
    }
}
