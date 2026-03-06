use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Severity level for toast notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    /// Get default duration for this level
    pub fn default_duration(&self) -> Duration {
        match self {
            ToastLevel::Info => Duration::from_secs(3),
            ToastLevel::Success => Duration::from_secs(3),
            ToastLevel::Warning => Duration::from_secs(5),
            ToastLevel::Error => Duration::from_secs(8),
        }
    }
}

/// A single toast notification
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn new(message: String, level: ToastLevel) -> Self {
        let duration = level.default_duration();
        Self {
            message,
            level,
            created_at: Instant::now(),
            duration,
        }
    }

    pub fn with_duration(message: String, level: ToastLevel, duration: Duration) -> Self {
        Self {
            message,
            level,
            created_at: Instant::now(),
            duration,
        }
    }

    /// Check if this toast has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.duration
    }

    /// Get progress from 0.0 (just created) to 1.0 (about to expire)
    /// Returns 1.0 if expired
    pub fn progress(&self) -> f32 {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.duration {
            return 1.0;
        }
        elapsed.as_secs_f32() / self.duration.as_secs_f32()
    }
}

/// Queue for managing multiple toast notifications
#[derive(Debug)]
pub struct ToastQueue {
    toasts: VecDeque<Toast>,
    max_capacity: usize,
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastQueue {
    pub fn new() -> Self {
        Self {
            toasts: VecDeque::new(),
            max_capacity: 10,
        }
    }

    /// Push a new toast to the queue
    pub fn push(&mut self, toast: Toast) {
        if self.toasts.len() >= self.max_capacity {
            // Remove oldest toast
            self.toasts.pop_front();
        }
        self.toasts.push_back(toast);
    }

    /// Convenience method to push a toast by message and level
    pub fn info(&mut self, message: impl Into<String>) {
        self.push(Toast::new(message.into(), ToastLevel::Info));
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.push(Toast::new(message.into(), ToastLevel::Success));
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(Toast::new(message.into(), ToastLevel::Warning));
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Toast::new(message.into(), ToastLevel::Error));
    }

    /// Remove and return expired toasts
    pub fn pop_expired(&mut self) -> Vec<Toast> {
        let mut expired = Vec::new();
        while let Some(front) = self.toasts.pop_front() {
            if front.is_expired() {
                expired.push(front);
            } else {
                // Put it back (it's the first non-expired)
                self.toasts.push_front(front);
                break;
            }
        }
        expired
    }

    /// Get an iterator over current toasts (not expired)
    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter()
    }

    /// Check if there are any toasts
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Get the number of toasts
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Clear all toasts
    pub fn clear(&mut self) {
        self.toasts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_creation() {
        let toast = Toast::new("Hello".to_string(), ToastLevel::Info);
        assert!(!toast.is_expired());
        assert_eq!(toast.message, "Hello");
    }

    #[test]
    fn test_toast_queue_push() {
        let mut queue = ToastQueue::new();
        queue.push(Toast::new("Test".to_string(), ToastLevel::Info));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_toast_queue_max_capacity() {
        let mut queue = ToastQueue::new();
        for i in 0..15 {
            queue.push(Toast::new(format!("Toast {}", i), ToastLevel::Info));
        }
        // Should keep last 10
        assert_eq!(queue.len(), 10);
    }

    #[test]
    fn test_toast_queue_pop_expired() {
        let mut queue = ToastQueue::new();

        // Push an expired toast
        let mut toast =
            Toast::with_duration("Expired".to_string(), ToastLevel::Info, Duration::ZERO);
        // Manually backdate (trick it)
        toast = Toast {
            created_at: Instant::now() - Duration::from_secs(10),
            ..toast
        };
        queue.push(toast);

        // Push a fresh toast
        queue.push(Toast::new("Fresh".to_string(), ToastLevel::Info));

        let expired = queue.pop_expired();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].message, "Expired");
        assert_eq!(queue.len(), 1);
    }
}
