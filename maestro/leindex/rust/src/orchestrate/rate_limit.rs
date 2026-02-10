use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    pub is_limited: bool,
    pub consecutive_hits: u32,
    pub last_hit_at: Option<u64>,   // Unix timestamp in seconds
    pub backoff_until: Option<u64>, // Unix timestamp in seconds
    pub last_message: Option<String>,
    pub last_retry_after: Option<u64>,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            is_limited: false,
            consecutive_hits: 0,
            last_hit_at: None,
            backoff_until: None,
            last_message: None,
            last_retry_after: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitBackoffOutcome {
    pub delay_secs: u64,
    pub used_retry_after: bool,
    pub exceeded_max: bool,
}

/// Tracks rate-limit state and computes exponential backoff (Ralph parity).
pub struct RateLimitBackoff {
    pub state: RateLimitState,
}

impl RateLimitBackoff {
    pub fn new() -> Self {
        Self {
            state: RateLimitState::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_hit(
        &mut self,
        message: Option<String>,
        retry_after: Option<u64>,
        max_consecutive_hits: u32,
        base_backoff_secs: u64,
        max_backoff_secs: u64,
    ) -> RateLimitBackoffOutcome {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.state.consecutive_hits += 1;
        self.state.last_hit_at = Some(now);
        self.state.last_message = message;
        self.state.last_retry_after = retry_after;

        let used_retry_after = retry_after.is_some();
        let backoff_duration = retry_after.unwrap_or_else(|| {
            // Exponential backoff with clamp
            let exp = base_backoff_secs.saturating_mul(
                2u64.pow(self.state.consecutive_hits.saturating_sub(1).min(16)), // clamp exponent
            );
            std::cmp::min(exp, max_backoff_secs)
        });

        self.state.is_limited = true;
        self.state.backoff_until = Some(now + backoff_duration);

        RateLimitBackoffOutcome {
            delay_secs: backoff_duration,
            used_retry_after,
            exceeded_max: self.state.consecutive_hits > max_consecutive_hits,
        }
    }

    pub fn reset(&mut self) {
        self.state.is_limited = false;
        self.state.consecutive_hits = 0;
        self.state.backoff_until = None;
        self.state.last_hit_at = None;
        self.state.last_message = None;
        self.state.last_retry_after = None;
    }

    pub fn check_backoff(&mut self) -> bool {
        if !self.state.is_limited {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(until) = self.state.backoff_until {
            if now >= until {
                self.state.is_limited = false;
                // We don't reset consecutive_hits here to allow for escalating backoff if it hits again immediately
                return false;
            }
            return true;
        }

        false
    }
}
