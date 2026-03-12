//! Context Engine for Orchestrate
//!
//! Encapsulates LeIndex-powered codebase analysis and pruning logic
//! to provide high-quality context within token budgets.

use crate::five_phase::{
    phase1_structural_scan, phase2_dependency_map, phase3_logic_flow, phase4_critical_path,
    phase5_optimization_report, PhaseOptions,
};
use crate::orchestrate::model::TrackPlan;
use crate::token_format::FormatMode;
use anyhow::Result;

pub struct ContextEngine {
    budget: usize,
}

impl ContextEngine {
    pub fn new(budget: usize) -> Self {
        Self { budget }
    }

    /// Generate a context bundle for the given track using all 5 phases with prioritization
    pub fn build_context(&self, tracks_dir: &std::path::Path, plan: &TrackPlan) -> Result<String> {
        // Skip LeIndex if context budget is too low (< 10K tokens)
        const MIN_BUDGET_FOR_LEINDEX: usize = 10000;
        if self.budget < MIN_BUDGET_FOR_LEINDEX {
            return Ok("// LeIndex disabled: context budget too low".to_string());
        }

        // Use the track's canonical root directory for LeIndex analysis
        let track_path = tracks_dir.join(&plan.track_id);
        let project_root = if track_path.exists() {
            track_path.clone()
        } else {
            tracks_dir.to_path_buf()
        };

        // Determine the analysis mode based on context budget
        let mode = if self.budget > 50000 {
            FormatMode::Balanced
        } else {
            FormatMode::Ultra
        };

        // Run 5-phase analysis with appropriate token limits
        // Cap max_files based on budget to prevent excessive scans
        let max_files = std::cmp::min(15, self.budget / 3000);

        let options = PhaseOptions {
            root: project_root,
            mode,
            max_files,
            max_focus_files: std::cmp::min(3, max_files / 5),
            top_n: 10,
            max_output_chars: self.budget / 5, // Default, will be overridden per phase
        };

        // Priority-based budget allocation (weights out of 100)
        // Structural: 25%
        // Dependency: 25%
        // Logic Flow: 10%
        // Critical Path: 30%
        // Optimization: 10%
        let weights = [25, 25, 10, 30, 10];

        let mut context = String::new();
        context.push_str("## Codebase Context (LeIndex 5-Phase Analysis)\n\n");

        // Phase 1: Structural Scan (Weight: 25%)
        let mut opt = options.clone();
        opt.max_output_chars = self.budget * weights[0] / 100;
        if let Ok(p1) = phase1_structural_scan(&opt) {
            context.push_str("### Phase 1: Structural Scan\n");
            context.push_str(&p1);
            context.push_str("\n\n");
        }

        // Phase 2: Dependency Map (Weight: 25%)
        opt.max_output_chars = self.budget * weights[1] / 100;
        if let Ok(p2) = phase2_dependency_map(&opt) {
            context.push_str("### Phase 2: Dependency Map\n");
            context.push_str(&p2);
            context.push_str("\n\n");
        }

        // Phase 3: Logic Flow (Weight: 10%)
        opt.max_output_chars = self.budget * weights[2] / 100;
        if let Ok(p3) = phase3_logic_flow(&opt) {
            context.push_str("### Phase 3: Logic Flow\n");
            context.push_str(&p3);
            context.push_str("\n\n");
        }

        // Phase 4: Critical Path (Weight: 30%)
        opt.max_output_chars = self.budget * weights[3] / 100;
        if let Ok(p4) = phase4_critical_path(&opt) {
            context.push_str("### Phase 4: Critical Path\n");
            context.push_str(&p4);
            context.push_str("\n\n");
        }

        // Phase 5: Optimization Report (Weight: 10%)
        opt.max_output_chars = self.budget * weights[4] / 100;
        if let Ok(p5) = phase5_optimization_report(&opt) {
            context.push_str("### Phase 5: Optimization Report\n");
            context.push_str(&p5);
            context.push_str("\n\n");
        }

        Ok(context)
    }
}
