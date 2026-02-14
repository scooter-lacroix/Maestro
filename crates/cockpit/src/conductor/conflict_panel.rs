//! Conflict resolution panel component

use leindex_core::orchestrate::model::{ConflictInfo, MergeStatus};

/// Resolution method for a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionMethod {
    AcceptOurs,
    AcceptTheirs,
    AiResolve,
    Skip,
}

impl Default for ResolutionMethod {
    fn default() -> Self {
        ResolutionMethod::AcceptOurs
    }
}

/// Conflict resolution panel
#[derive(Debug, Clone)]
pub struct ConflictPanel {
    /// The conflict being displayed
    pub conflict: Option<ConflictInfo>,
    /// Whether the panel is visible
    pub visible: bool,
    /// Selected resolution option
    pub selected_option: ResolutionMethod,
}

impl Default for ConflictPanel {
    fn default() -> Self {
        Self {
            conflict: None,
            visible: false,
            selected_option: ResolutionMethod::AcceptOurs,
        }
    }
}

impl ConflictPanel {
    /// Create a new conflict panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a conflict
    pub fn show(&mut self, conflict: ConflictInfo) {
        self.conflict = Some(conflict);
        self.visible = true;
        self.selected_option = ResolutionMethod::AcceptOurs;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.conflict = None;
    }

    /// Select next option
    pub fn next_option(&mut self) {
        self.selected_option = match self.selected_option {
            ResolutionMethod::AcceptOurs => ResolutionMethod::AcceptTheirs,
            ResolutionMethod::AcceptTheirs => ResolutionMethod::AiResolve,
            ResolutionMethod::AiResolve => ResolutionMethod::Skip,
            ResolutionMethod::Skip => ResolutionMethod::AcceptOurs,
        };
    }

    /// Select previous option
    pub fn prev_option(&mut self) {
        self.selected_option = match self.selected_option {
            ResolutionMethod::AcceptOurs => ResolutionMethod::Skip,
            ResolutionMethod::AcceptTheirs => ResolutionMethod::AcceptOurs,
            ResolutionMethod::AiResolve => ResolutionMethod::AcceptTheirs,
            ResolutionMethod::Skip => ResolutionMethod::AiResolve,
        };
    }

    /// Get label for current option
    pub fn get_option_label(&self) -> &'static str {
        match self.selected_option {
            ResolutionMethod::AcceptOurs => "Accept Ours (r)",
            ResolutionMethod::AcceptTheirs => "Accept Theirs (s)",
            ResolutionMethod::AiResolve => "AI Resolve (o)",
            ResolutionMethod::Skip => "Skip (t)",
        }
    }
}
