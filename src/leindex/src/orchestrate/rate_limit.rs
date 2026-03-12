use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct RateLimitState {
    pub is_limited: bool,
    pub consecutive_hits: u32,
    pub last_hit_at: Option<u64>,   // Unix timestamp in seconds
    pub backoff_until: Option<u64>, // Unix timestamp in seconds
    pub last_message: Option<String>,
    pub last_retry_after: Option<u64>,
}

/// Rate limit backoff state for a specific agent
#[derive(Debug, Clone)]
pub struct RateLimitBackoff {
    pub state: RateLimitState,
    pub retry_after: Option<u64>, // Unix timestamp
}

impl RateLimitBackoff {
    pub fn new() -> Self {
        Self {
            state: RateLimitState::default(),
            retry_after: None,
        }
    }

    #[allow(unused_variables)]
    pub fn record_hit(
        &mut self,
        message: Option<String>,
        retry_after: Option<u64>,
        max_retries: u32,
        base_secs: u64,
        max_secs: u64,
    ) -> BackoffOutcome {
        self.state.consecutive_hits += 1;
        self.state.last_hit_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        self.state.last_message = message;
        self.state.is_limited = self.state.consecutive_hits > max_retries;

        let exceeded_max = self.state.consecutive_hits > max_retries;
        self.state.is_limited = exceeded_max;

        if exceeded_max {
            let delay_secs = (base_secs * self.state.consecutive_hits as u64).min(max_secs);
            self.state.backoff_until = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + delay_secs,
            );
            self.retry_after = Some(delay_secs);
            BackoffOutcome {
                exceeded_max: true,
                delay_secs,
            }
        } else {
            BackoffOutcome {
                exceeded_max: false,
                delay_secs: 0,
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = RateLimitState::default();
        self.retry_after = None;
    }
}

impl Default for RateLimitBackoff {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackoffOutcome {
    pub exceeded_max: bool,
    pub delay_secs: u64,
}

impl BackoffOutcome {
    pub fn continue_outcome() -> Self {
        Self {
            exceeded_max: false,
            delay_secs: 0,
        }
    }

    pub fn backout(delay_secs: u64, exceeded_max: bool) -> Self {
        Self {
            exceeded_max,
            delay_secs,
        }
    }
}


