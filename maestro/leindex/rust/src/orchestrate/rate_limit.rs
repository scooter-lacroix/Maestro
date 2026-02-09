use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    pub is_limited: bool,
    pub consecutive_hits: u32,
    pub last_hit_at: Option<u64>, // Unix timestamp in seconds
    pub backoff_until: Option<u64>, // Unix timestamp in seconds
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            is_limited: false,
            consecutive_hits: 0,
            last_hit_at: None,
            backoff_until: None,
        }
    }
}

pub struct RateLimitDetector {
    pub state: RateLimitState,
    max_consecutive_hits: u32,
    base_backoff_secs: u64,
}

impl RateLimitDetector {
    pub fn new(max_consecutive_hits: u32, base_backoff_secs: u64) -> Self {
        Self {
            state: RateLimitState::default(),
            max_consecutive_hits,
            base_backoff_secs,
        }
    }

    pub fn record_hit(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.state.consecutive_hits += 1;
        self.state.last_hit_at = Some(now);

        if self.state.consecutive_hits >= self.max_consecutive_hits {
            self.state.is_limited = true;
            // Exponential backoff or simple fixed backoff
            let backoff_duration = self.base_backoff_secs * 2u64.pow(self.state.consecutive_hits.saturating_sub(self.max_consecutive_hits));
            self.state.backoff_until = Some(now + backoff_duration);
        }
    }

    pub fn reset(&mut self) {
        self.state.is_limited = false;
        self.state.consecutive_hits = 0;
        self.state.backoff_until = None;
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
