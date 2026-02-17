use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadState {
    Processing,
    AwaitingApproval,
    AwaitingAuth,
    Completed,
    Failed,
}
