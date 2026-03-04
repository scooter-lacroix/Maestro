// TrackLens Pane for Cockpit TUI
//
// This module provides the TrackLens pane for the Cockpit terminal UI:
// - Active review indicator
// - Review history
// - Integration with TrackLens server

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

// Import ReviewMode from leindex-core to avoid duplication
use leindex_core::tracklens::ReviewMode;

// ─── TrackLens Pane ───────────────────────────────────────────────────────────

/// TrackLens review status for the TUI
#[derive(Debug, Clone, Default)]
pub struct TrackLensPane {
    /// Whether a review is currently active
    pub active: bool,
    /// Current review information
    pub current_review: Option<ReviewStatus>,
    /// Review history
    pub history: Vec<ReviewHistoryEntry>,
}

/// Status of the current review
#[derive(Debug, Clone)]
pub struct ReviewStatus {
    /// Track ID being reviewed
    pub track_id: String,
    /// Document type (spec, plan, walkthrough)
    pub document_type: String,
    /// Review mode
    pub mode: ReviewMode,
    /// Server URL
    pub server_url: String,
    /// When the review started
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Entry in review history
#[derive(Debug, Clone)]
pub struct ReviewHistoryEntry {
    /// Track ID
    pub track_id: String,
    /// Document type
    pub document_type: String,
    /// Whether it was approved
    pub approved: bool,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Number of annotations
    pub annotation_count: usize,
}

impl TrackLensPane {
    /// Create a new TrackLens pane
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new review
    pub fn start_review(&mut self, track_id: String, document_type: String, mode: ReviewMode, server_url: String) {
        self.active = true;
        self.current_review = Some(ReviewStatus {
            track_id,
            document_type,
            mode,
            server_url,
            started_at: chrono::Utc::now(),
        });
    }

    /// Complete the current review
    pub fn complete_review(&mut self, approved: bool, annotation_count: usize) {
        if let Some(review) = self.current_review.take() {
            self.history.push(ReviewHistoryEntry {
                track_id: review.track_id,
                document_type: review.document_type,
                approved,
                timestamp: chrono::Utc::now(),
                annotation_count,
            });
            self.active = false;
        }
    }

    /// Render the TrackLens pane
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(area);

        // Header
        let header_style = if self.active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Gray)
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled("TrackLens", header_style),
            Span::raw(if self.active { " (Active)" } else { " (Idle)" }),
        ]))
        .block(Block::default().borders(Borders::ALL));

        frame.render_widget(header, chunks[0]);

        // Content
        if self.active {
            if let Some(ref review) = self.current_review {
                let content = vec![
                    ListItem::new(format!("Track: {}", review.track_id)),
                    ListItem::new(format!("Document: {}", review.document_type)),
                    ListItem::new(format!("Mode: {:?}", review.mode)),
                    ListItem::new(format!("URL: {}", review.server_url)),
                ];

                let list = List::new(content)
                    .block(Block::default().title("Current Review").borders(Borders::ALL));

                frame.render_widget(list, chunks[1]);
            }
        } else if !self.history.is_empty() {
            let items: Vec<ListItem> = self
                .history
                .iter()
                .rev()
                .take(10)
                .map(|entry| {
                    let status = if entry.approved { "✓" } else { "✗" };
                    ListItem::new(format!(
                        "{} {} - {} ({})",
                        status,
                        entry.track_id,
                        entry.document_type,
                        entry.annotation_count
                    ))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().title("Review History").borders(Borders::ALL));

            frame.render_widget(list, chunks[1]);
        } else {
            let text = Paragraph::new("No reviews yet")
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: true });

            frame.render_widget(text, chunks[1]);
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_creation() {
        let pane = TrackLensPane::new();
        assert!(!pane.active);
        assert!(pane.current_review.is_none());
        assert!(pane.history.is_empty());
    }

    #[test]
    fn test_start_review() {
        let mut pane = TrackLensPane::new();
        pane.start_review(
            "test-track".to_string(),
            "spec".to_string(),
            ReviewMode::Review,
            "http://localhost:3000".to_string(),
        );

        assert!(pane.active);
        assert!(pane.current_review.is_some());
        assert_eq!(pane.current_review.as_ref().unwrap().track_id, "test-track");
    }

    #[test]
    fn test_complete_review() {
        let mut pane = TrackLensPane::new();
        pane.start_review(
            "test-track".to_string(),
            "spec".to_string(),
            ReviewMode::Review,
            "http://localhost:3000".to_string(),
        );

        pane.complete_review(true, 2);

        assert!(!pane.active);
        assert!(pane.current_review.is_none());
        assert_eq!(pane.history.len(), 1);
        assert!(pane.history[0].approved);
        assert_eq!(pane.history[0].annotation_count, 2);
    }
}
