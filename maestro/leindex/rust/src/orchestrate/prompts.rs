//! Prompt templates for orchestrate modes
//!
//! Provides planning and building mode prompt templates.

use crate::orchestrate::model::{LoopMode, Task, TrackPlan, SessionState};
use anyhow::Result;

/// Prompt template builder
pub struct PromptBuilder {
    context_budget: usize,
}

impl PromptBuilder {
    pub fn new(context_budget: usize) -> Self {
        Self { context_budget }
    }

    /// Build a prompt for the current iteration
    pub fn build_prompt(
        &self,
        task: &Task,
        session: &SessionState,
        plan: &TrackPlan,
        recent_iterations: &[crate::orchestrate::model::IterationLog],
        leindex_context: Option<&str>,
<<<<<<< HEAD
        memory_context: Option<&str>,
=======
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
    ) -> Result<String> {
        let mut prompt = String::new();
        let mut budget_used = 0;

        // Add track context
        prompt.push_str("# Track Context\n\n");
        prompt.push_str(&format!("**Track ID:** {}\n", plan.track_id));
        prompt.push_str(&format!("**Mode:** {:?}\n\n", session.mode));
        budget_used += prompt.len();

        // Add recent progress summary (limit to 3 most recent to save tokens)
        if !recent_iterations.is_empty() {
            prompt.push_str("## Recent Progress\n\n");
            for log in recent_iterations.iter().rev().take(3) {
                let status_emoji = match log.status {
                    crate::orchestrate::model::IterationStatus::Completed => "✓",
                    crate::orchestrate::model::IterationStatus::Failed => "✗",
                    crate::orchestrate::model::IterationStatus::Running => "→",
                    crate::orchestrate::model::IterationStatus::Skipped => "○",
                };
                prompt.push_str(&format!(
                    "- [{}] **Iteration {}**: {} ({})\n",
                    status_emoji,
                    log.iteration,
                    log.task_id,
                    format!("{:?}", log.status).to_lowercase()
                ));
            }
            prompt.push('\n');
            budget_used += prompt.len() - budget_used;
        }

<<<<<<< HEAD
        // Add memory context (LSP diagnostics, recent observations)
        if let Some(memory) = memory_context {
            if !memory.trim().is_empty() {
                prompt.push_str("## Memory Context\n\n");
                prompt.push_str(memory);
                if !memory.ends_with('\n') {
                    prompt.push('\n');
                }
                prompt.push('\n');
            }
        }

=======
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
        // Add current task details
        prompt.push_str("## Current Task\n\n");
        prompt.push_str(&format!("**Task:** {}\n", task.title));
        prompt.push_str(&format!("**Status:** {:?}\n", task.status));

        if !task.description.is_empty() {
            prompt.push_str(&format!("\n**Description:**\n{}\n", task.description));
        }

        // Add subtasks if present (limit display to save tokens)
        if !task.subtasks.is_empty() {
            prompt.push_str("\n### Subtasks\n\n");
            // Show at most 10 subtasks to avoid excessive prompt size
            for subtask in task.subtasks.iter().take(10) {
                let status_marker = match subtask.status {
                    crate::orchestrate::model::TrackStatus::Pending => "[ ]",
                    crate::orchestrate::model::TrackStatus::InProgress => "[~]",
                    crate::orchestrate::model::TrackStatus::Completed => "[x]",
                };
                prompt.push_str(&format!("{} {}\n", status_marker, subtask.title));
            }
            if task.subtasks.len() > 10 {
                prompt.push_str(&format!("... ({} more subtasks)\n", task.subtasks.len() - 10));
            }
        }
        budget_used += prompt.len() - budget_used;

        // Add mode-specific instructions
        prompt.push_str("\n## Instructions\n\n");
        match session.mode {
            LoopMode::Planning => {
                prompt.push_str(self.planning_instructions());
            }
            LoopMode::Building => {
                prompt.push_str(&self.building_instructions(task));
            }
        }
        budget_used += prompt.len() - budget_used;

        // Calculate remaining budget for LeIndex context
<<<<<<< HEAD
        // Ensure core instructions are never truncated by reserving 8KB
        let reserved = 8192;
        let remaining_budget = self.context_budget.saturating_sub(budget_used + reserved);

        // Add LeIndex context if provided (truncate if necessary)
        if let Some(context) = leindex_context {
            if !context.is_empty() && remaining_budget > 1000 {
=======
        // Reserve ~25% of budget for the task instructions we just added
        let remaining_budget = self.context_budget.saturating_sub(budget_used * 2);

        // Add LeIndex context if provided (truncate if necessary)
        if let Some(context) = leindex_context {
            if !context.is_empty() {
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
                prompt.push_str("\n## Codebase Context (LeIndex)\n\n");

                // Truncate context if it would exceed our budget
                let context_to_add = if context.len() > remaining_budget {
                    // Truncate from the end, keeping the beginning (usually has file list)
                    let mut truncated = String::from(&context[..remaining_budget]);
<<<<<<< HEAD
                    truncated.push_str("\n\n... (Context truncated to fit budget)");
=======
                    truncated.push_str("\n\n... (Context truncated to fit token budget)");
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
                    truncated
                } else {
                    context.to_string()
                };

                prompt.push_str(&context_to_add);
                prompt.push('\n');
            }
        }

        Ok(prompt)
    }

    /// Planning mode instructions
    fn planning_instructions(&self) -> &'static str {
        r#"You are in **PLANNING MODE**. Your role is to analyze and plan, NOT to implement code.

### Your Responsibilities:

1. **Analyze the current state** of the codebase using the provided LeIndex context
2. **Generate or update the plan** (plan.md) with detailed task breakdowns
<<<<<<< HEAD
3. **Bank key discoveries** as memories (e.g., "Found existing auth logic in middleware.py")
4. **Identify dependencies** between tasks
5. **Estimate complexity** and prioritize tasks
6. **Document architectural decisions** and technical considerations
=======
3. **Identify dependencies** between tasks
4. **Estimate complexity** and prioritize tasks
5. **Document architectural decisions** and technical considerations
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)

### What NOT To Do:

- ❌ Do NOT write implementation code
- ❌ Do NOT make commits
- ❌ Do NOT run tests or build commands

### What To Do Instead:

- ✅ Use LeIndex to understand existing code structure
<<<<<<< HEAD
- ✅ Bank memories for key discoveries or architectural decisions
=======
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
- ✅ Break down the task into clear, actionable subtasks
- ✅ Identify what needs to be created/modified/deleted
- ✅ Consider edge cases and error handling
- ✅ Suggest testing strategies

### Output Format:

Update the plan.md file with your analysis. Use the following format:

```markdown
### [x] Task X.Y: Brief description

**Analysis:**
- Current state: [what exists now]
- Required changes: [what needs to change]
- Dependencies: [what this depends on]
- Risk assessment: [potential issues]

**Subtasks:**
- [ ] Subtask 1
- [ ] Subtask 2
```

When done, respond with `<promise>COMPLETE</promise>` and a brief summary of your plan.
"#
    }

    /// Building mode instructions
    fn building_instructions(&self, task: &Task) -> String {
        format!(
            r#"You are in **BUILDING MODE**. Your role is to implement exactly ONE task.

## Current Task

**Title:** {}

### Implementation Requirements:

1. **Implement ONLY this task** - do not add features beyond the scope
<<<<<<< HEAD
2. **Bank a summary memory** of your implementation upon completion
3. **Follow best practices** for the language/framework
4. **Write tests** for new functionality
5. **Run validation** - tests, linters, type checkers
6. **Update documentation** if needed
=======
2. **Follow best practices** for the language/framework
3. **Write tests** for new functionality
4. **Run validation** - tests, linters, type checkers
5. **Update documentation** if needed
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)

### Completion Criteria:

Your work is COMPLETE when ALL of the following are true:

<<<<<<< HEAD
1. ✅ Implementation is done and **VERIFIED via actual code execution** (tests, analysis)
2. ✅ **Evidence of verification** is provided in the output (e.g., test results, log snippets)
3. ✅ **A memory is banked** summarizing key changes or new knowledge
4. ✅ Code follows project conventions
5. ✅ **plan.md is updated** - mark this task as `[x]`
6. ✅ **Changes are committed** with a descriptive commit message
7. ✅ Respond with `<promise>COMPLETE</promise>`

**CRITICAL:** Do NOT mark a task as complete based on a claim. You MUST run the code, see it work, and include that proof in your response. The orchestrate loop will NOT exit until every task in the track is physically verified.
=======
1. ✅ Implementation is done and tests pass
2. ✅ Code follows project conventions
3. ✅ **plan.md is updated** - mark this task as `[x]`
4. ✅ **Changes are committed** with a descriptive commit message
5. ✅ Respond with `<promise>COMPLETE</promise>`
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)

### Commit Message Format:

```
{}: <brief description>

- Implemented {}
- Tests: <pass/fail>
- Notes: <any additional info>
```

### Before Completing:

- Run all tests: `cargo test` / `pytest` / `npm test`
- Check code quality: linters, formatters
- Verify the plan.md is updated
- Make sure everything is committed

### If You Get Stuck:

1. Use LeIndex context to understand existing patterns
2. Follow similar code already in the codebase
3. If blocked by a real issue, document it and ask for help
4. Never skip tests or validation

**Remember:** Quality over speed. A single well-implemented task is better than many half-done ones.
"#,
            task.title,
            task.id,
            task.title
        )
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new(50000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrate::model::{TrackStatus, SessionStatus};
    use chrono::Utc;

    #[test]
    fn test_planning_prompt() {
        let builder = PromptBuilder::new(50000);

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TrackStatus::Pending,
            dependencies: vec![],
            description: "A test task".to_string(),
            subtasks: vec![],
            notes: None,
            line_number: 0,
        };

        let session = SessionState {
            track_id: "test-track".to_string(),
            mode: LoopMode::Planning,
            agent_config: Default::default(),
            current_iteration: 1,
            current_task_id: Some("test-task".to_string()),
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            status: SessionStatus::Running,
        };

        let plan = TrackPlan {
            track_id: "test-track".to_string(),
            tasks: vec![],
            phases: vec![],
        };

<<<<<<< HEAD
        let prompt = builder
            .build_prompt(&task, &session, &plan, &[], None, None)
            .unwrap();
=======
        let prompt = builder.build_prompt(&task, &session, &plan, &[], None).unwrap();
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)

        assert!(prompt.contains("PLANNING MODE"));
        assert!(prompt.contains("NOT to implement code"));
        assert!(prompt.contains("plan.md"));
    }

    #[test]
    fn test_building_prompt() {
        let builder = PromptBuilder::new(50000);

        let task = Task {
            id: "test-task".to_string(),
            title: "Implement Feature X".to_string(),
            status: TrackStatus::Pending,
            dependencies: vec![],
            description: "Build feature X".to_string(),
            subtasks: vec![],
            notes: None,
            line_number: 0,
        };

        let session = SessionState {
            track_id: "test-track".to_string(),
            mode: LoopMode::Building,
            agent_config: Default::default(),
            current_iteration: 1,
            current_task_id: Some("test-task".to_string()),
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            status: SessionStatus::Running,
        };

        let plan = TrackPlan {
            track_id: "test-track".to_string(),
            tasks: vec![],
            phases: vec![],
        };

<<<<<<< HEAD
        let prompt = builder
            .build_prompt(&task, &session, &plan, &[], None, None)
            .unwrap();
=======
        let prompt = builder.build_prompt(&task, &session, &plan, &[], None).unwrap();
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)

        assert!(prompt.contains("BUILDING MODE"));
        assert!(prompt.contains("Implement Feature X"));
        assert!(prompt.contains("plan.md is updated"));
        assert!(prompt.contains("<promise>COMPLETE</promise>"));
    }
}
