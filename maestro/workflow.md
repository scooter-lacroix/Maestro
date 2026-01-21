# Project Workflow

## Guiding Principles

1. **The Plan is the Source of Truth:** All work must be tracked in `plan.md`
2. **The Tech Stack is Deliberate:** Changes to the tech stack must be documented in `tech-stack.md` *before* implementation
3. **Test-Driven Development:** Write unit tests before implementing functionality
4. **High Code Coverage:** Aim for >98% code coverage for all modules
5. **User Experience First:** Every decision should prioritize user experience
6. **Non-Interactive & CI-Aware:** Prefer non-interactive commands. Use `CI=true` for watch-mode tools (tests, linters) to ensure single execution.
7. **Critical Think First:** Apply systematic metacognitive analysis before and after key actions to ensure quality and prevent common pitfalls.

## Critical Think Integration

**MANDATORY REQUIREMENT: Apply the Critical Think metacognitive framework at strategic workflow points.**

The 6-Step Critical Think Framework:
1. **Core Thesis & Confidence Score (1-10)**
2. **Foundational Analysis** (top 3 assumptions)
3. **Logical Integrity Check** (identify fallacies)
4. **AI-Specific Pitfall Analysis** (problem evasion, happy path bias, over-engineering, hallucination)
5. **Risk & Mitigation** (identify risks and mitigations)
6. **Synthesis & Revised Recommendation** (updated confidence and action plan)

### Integration Points

**1. Before Asking Clarifying Questions (newTrack Phase)**
- Use the `criticalthink_question.md` template
- Analyze whether the question is truly necessary
- Check for authority bias (asking when you could decide)
- Avoid over-questioning

**2. After Receiving User Answers (newTrack Phase)**
- Use the `criticalthink_after_action.md` template
- Validate understanding of the answer
- Check if assumptions hold
- Identify gaps before proceeding

**3. Before Generating Documentation (spec.md, plan.md)**
- Use the `criticalthink_docs.md` template
- Verify all claims will be accurate
- Plan structure and completeness
- Avoid happy path bias (document error scenarios)

**4. After Generating Documentation**
- Use the `criticalthink_after_action.md` template
- Verify technical accuracy
- Check code examples
- Validate completeness

**5. Before Code Implementation**
- Use the `criticalthink_implementation.md` template
- Analyze approach and assumptions
- Identify edge cases
- Plan error handling

**6. After Code Implementation**
- Use the `criticalthink_after_action.md` template
- Validate implementation quality
- Check for bugs
- Verify test coverage

**7. Before Agent Delegation**
- Use the `criticalthink_agent_delegation.md` template
- Verify delegation is appropriate
- Check for authority bias
- Ensure agent has required capabilities

**8. After Agent Delegation**
- Use the `criticalthink_after_action.md` template
- Validate agent results
- Check quality of deliverables
- Assess agent performance

### Confidence Score Thresholds

- **1-4 (Critical):** Reconsider approach entirely
- **5-6 (Warning):** Proceed with caution, document risks
- **7-8 (Acceptable):** Proceed with standard approach
- **9-10 (High):** Highly confident, minimal oversight needed

### Configuration

Critical Think behavior can be configured in `maestro/critical_think/config.yaml`:
- Enable/disable specific integration points
- Adjust confidence thresholds
- Control output verbosity
- Set auto-proceed behavior

### Templates Location

Templates are stored in `maestro/critical_think/templates/`:
- `criticalthink_before_action.md` - Generic pre-action analysis
- `criticalthink_after_action.md` - Generic post-action validation
- `criticalthink_question.md` - Q&A phase specialized
- `criticalthink_docs.md` - Documentation generation specialized
- `criticalthink_implementation.md` - Code implementation specialized
- `criticalthink_agent_delegation.md` - Agent delegation specialized

## Agent Usage Requirements

**CRITICAL SYSTEM DIRECTIVE: PROACTIVE AUTOMATIC AGENT USAGE**

1. **Automatic Agent Selection is MANDATORY:** Agent usage is the DEFAULT behavior, NOT an optional enhancement. You MUST proactively select and launch agents based on task complexity WITHOUT waiting for user instruction. The user has configured Maestro to use agents automatically.

2. **NEVER Ask for Agent Permission:** Do NOT ask "Should I use an agent?" or "Which agent should I use?". Make the decision automatically based on task complexity and launch the appropriate agents. Agent selection is YOUR responsibility, not the user's.

3. **Agent Selection Criteria (Execute Automatically):**
   - **Trivial tasks (1-5 lines, simple changes):** Implement directly
   - **Standard tasks (5-50 lines, single file):** Automatically use appropriate implementation agents (explore)
   - **Complex tasks (multiple files, >50 lines):** Automatically use oracle or librarian for design + appropriate implementation agents
   - **Large codebase analysis (>100KB):** Automatically use librarian for exploration
   - **Spec-driven/ambiguous requirements:** Automatically use oracle for specification
   - **ALL implementation work:** MUST be automatically followed by oracle for validation

**Core Agents:**
- **oracle**: Architecture, code review, strategy. (MANDATORY for all implementation)
- **librarian**: Multi-repo analysis, doc lookup, implementation examples.
- **explore**: Fast codebase exploration and pattern matching.
- **frontend-ui-ux-engineer**: Designer turned developer. Builds gorgeous UIs.
- **document-writer**: Technical writing expert. Writes prose that flows.
- **multimodal-looker**: Visual content specialist. Analyzes PDFs, images, diagrams.

**Orchestrator Agents:**
- **oracle**: Specialized in spec-driven development and strategic planning.
- **kilocode-orchestrator**: Large-scale projects with persistent memory across sessions.
- **llm-council-evaluator**: Meta-agent selection for high-risk or complex decisions.

4. **Mandatory Pre-Commit Review:** Before marking any task complete and committing:
   - ALL code changes MUST be automatically reviewed by oracle agent
   - Review results MUST be addressed before proceeding
   - If critical issues are found, they MUST be fixed before commit

5. **Quota Awareness:**
   - librarian: 300 requests/day (use sparingly for large analysis)
   - frontend-ui-ux-engineer: Unlimited free (use liberally for prototyping)
   - document-writer: Separate credit pool (use to preserve main quotas)
   - explore: 2000 requests/day (use for standard implementation)

## Agent Availability and Fallbacks

### Checking Tool Availability

Before using specialized agents that require external CLI tools, you MUST check if the required tools are available on the user's system.

**Check Command:**
```bash
# Check if perspective CLI tools are available
which gemini-cli 2>/dev/null && which qwen-cli 2>/dev/null
```

### Fallback Options

If external CLI tools are NOT available, you MUST:

1. **Recommend Installation:** Present the user with installation instructions:
   > "Some specialized agents require external CLI tools for optimal performance.
   > Would you like to:
   > A) Install the CLI tools (recommended for full functionality)
   > B) Use built-in Claude Code agents (limited but functional)
   >
   > To install CLI tools, visit: https://github.com/scooter-lacroix/Council-of-Agents"

2. **Use Built-in Alternatives:** If user chooses option B, continue using:
   - Built-in Claude Code reasoning (no external tools needed)
   - Native Claude Code subagent capability
   - Standard model capabilities

3. **Document Decision:** Note the user's choice in the project context for future reference.

### Tool-Specific Requirements

- **librarian**: Requires `gemini-cli` with API keys
- **explore**: Requires `qwen-cli` with API keys
- **frontend-ui-ux-engineer**: May require `perspective-cli` framework
- **oracle**: Built-in (no external tools required)
- **document-writer**: May require `perspective-cli` framework

### Graceful Degradation

If an external agent is unavailable but recommended:
1. Inform the user which agent is unavailable
2. Provide the recommended alternative
3. Ask if they want to proceed with the alternative
4. Continue work with user's approval

## Task Workflow

All tasks follow a strict lifecycle:

### Standard Task Workflow

**AUTOMATIC AGENT SELECTION:**
- You MUST assess task complexity and launch appropriate agents automatically
- Do NOT await user instruction - agent usage is automatic and proactive
- See "Agent Selection Criteria" above for specific guidance

1. **Select Task:** Choose the next available task from `plan.md` in sequential order

2. **Mark In Progress:** Before beginning work, edit `plan.md` and change the task from `[ ]` to `[~]`

3. **Assess Complexity and Select Agent (AUTOMATIC):**
   - **CRITICAL:** Assess task complexity and automatically select the appropriate approach:
     - Trivial tasks (1-5 lines): Implement directly
     - Standard tasks (5-50 lines, single file): Automatically launch explore for implementation
     - Complex tasks (multiple files, >50 lines): Automatically launch oracle or librarian for design + explore for implementation
   - **Do NOT ask user permission** - this is automatic

4. **Use LeIndex for Code Exploration (MANDATORY):**
   - **CRITICAL:** Before writing any code, use `maestro:leindex` to understand the codebase
   - **For implementation tasks:** Extract context for relevant files using balanced mode (82% savings, LLM actionable)
   - **For exploration tasks:** Use ultra mode only (98% savings, exploration only)
   - **Required:** Run LeIndex analysis on files you'll be modifying:

   ```python
   from maestro.leindex import ContextExtractor

   # For code generation (default balanced mode)
   extractor = ContextExtractor(mode='balanced')
   result = extractor.extract_for_file('path/to/file.py')
   context = result.context.to_llm_string() if result else ""

   # For exploration/cross-file analysis
   from maestro.leindex import get_relevant_context
   context = get_relevant_context('/path/to/project', 'entry_point')

   # For call graph analysis
   from maestro.leindex import CallGraphAnalyzer
   cg_analyzer = CallGraphAnalyzer()
   graph = cg_analyzer.build_project_graph('/path/to/project')
   ```

   - **Purpose:** This step provides:
     - Understanding of existing code structure (what functions exist, their signatures)
     - Call chain analysis (what calls what, impact of changes)
     - Line numbers for navigation
     - 82% token savings vs raw file reads while preserving semantic completeness
     - Full function signatures (params, return types) needed for LLM to use the code

   - **NOT for:** Looking up documentation or external references - use appropriate tools for those

5. **Write Failing Tests (Red Phase):**
   - Create a new test file for the feature or bug fix.
   - Write one or more unit tests that clearly define the expected behavior and acceptance criteria for the task.
   - **CRITICAL:** Run the tests and confirm that they fail as expected. This is the "Red" phase of TDD. Do not proceed until you have failing tests.
   - **AUTOMATIC AGENT:** For test writing, automatically use appropriate agent (explore) for standard/complex tasks

5. **Implement to Pass Tests (Green Phase):**
   - Write the minimum amount of application code necessary to make the failing tests pass.
   - Run the test suite again and confirm that all tests now pass. This is the "Green" phase.
   - **AUTOMATIC AGENT:** For implementation, automatically use appropriate agent (explore) or oracle/librarian + explore (complex)

6. **Refactor (Optional but Recommended):**
   - With the safety of passing tests, refactor the implementation code and the test code to improve clarity, remove duplication, and enhance performance without changing the external behavior.
   - Rerun tests to ensure they still pass after refactoring.
   - **AUTOMATIC AGENT:** Automatically use explore for refactoring

7. **Verify Coverage:** Run coverage reports using the project's chosen tools. For example, in a Python project, this might look like:
   ```bash
   pytest --cov=app --cov-report=html
   ```
   Target: >98% coverage for new code. The specific tools and commands will vary by language and framework.

8. **Document Deviations:** If implementation differs from tech stack:
   - **STOP** implementation
   - Update `tech-stack.md` with new design
   - Add dated note explaining the change
   - Resume implementation

9. **Agent Review (MANDATORY - AUTOMATIC):**
   **CRITICAL:** Before proceeding to commit, you MUST automatically launch oracle. Do NOT wait for user instruction.
   - **AUTOMATICALLY Launch Code Review:** Use the oracle agent to review all changes made during this task. Provide context: task description, files changed, expected outcomes.
   - **Address Review Findings:** If critical issues are found, fix them before proceeding. If suggestions are provided, address or document decision to defer.
   - **Confirm Review Complete:** Only after oracle passes should you proceed to commit. Document any issues found and resolved.

10. **Commit Code Changes:**
    - Stage all code changes related to the task.
    - Propose a clear, concise commit message e.g, `feat(ui): Create basic HTML structure for calculator`.
    - Perform the commit.

11. **Create Obsidian Note for Task Summary:**
    - **Step 11.1: Get Commit Hash:** Obtain the hash of the *just-completed commit* (`git log -1 --format="%H"`).
    - **Step 11.2: Draft Note Content:** Create a detailed summary for the completed task. This should include the task name, a summary of changes, a list of all created/modified files, and the core "why" for the change.
    - **Step 11.3: Create Obsidian Note:** Write the summary as a new Obsidian markdown note in the configured vault.
      ```bash
      # Example: Create note in Obsidian vault
      OBSIDIAN_VAULT="${OBSIDIAN_VAULT_PATH:-$HOME/Documents/ObsidianVault}"
      NOTE_PATH="$OBSIDIAN_VAULT/Tasks/$(date +%Y%m%d)-task-summary.md"
      mkdir -p "$(dirname "$NOTE_PATH")"
      cat > "$NOTE_PATH" << EOF
      ---
      title: Task Summary - $(date +%Y-%m-%d)
      commit: $COMMIT_HASH
      task: $TASK_NAME
      date: $(date -Iseconds)
      tags: [task, completed, maestro]
      ---

      # Task: $TASK_NAME

      **Commit:** \`$COMMIT_HASH\`

      ## Summary
      [Task summary here]

      ## Changes
      - [File 1]
      - [File 2]

      ## Rationale
      [Why this change was made]
      EOF
      ```
      **Note:** Set the `OBSIDIAN_VAULT_PATH` environment variable to your Obsidian vault location.

12. **Get and Record Task Commit SHA:**
    - **Step 12.1: Update Plan:** Read `plan.md`, find the line for the completed task, update its status from `[~]` to `[x]`, and append the first 7 characters of the *just-completed commits commit hash.
    - **Step 12.2: Write Plan:** Write the updated content back to `plan.md`.

13. **Commit Plan Update:**
    - **Action:** Stage the modified `plan.md` file.
    - **Action:** Commit this change with a descriptive message (e.g., `maestro(plan): Mark task 'Create user model' as complete`).

---

## Phase Completion: "Tzar of Excellence" Review

**MANDATORY REQUIREMENT: At the completion of EACH phase, a rigorous zero-tolerance code review MUST be conducted.**

### When to Trigger

After ALL tasks in a phase are marked complete `[x]` in `plan.md`, BEFORE moving to the next phase, you MUST:

1. **Deploy codex-reviewer with "Tzar of Excellence" directive**
2. **Wait for review completion**
3. **Address all critical findings**
4. **Only then proceed to next phase**

### "Tzar of Excellence" Directive Template

When invoking codex-reviewer at phase completion, use the following directive:

```
You are conducting the "Tzar of Excellence" review for Phase [X] of the Maestro 2.0 unified development framework track.

## Zero Tolerance Excellence Directive

You are reviewing a completed phase with ZERO tolerance for:
- Mediocrity
- Corner cases unhandled
- Missing error handling
- Security vulnerabilities
- Poor performance
- Incomplete implementations
- Technical debt
- Code quality issues

## Review Scope

Review ALL code changes made during this phase:
- All commits in this phase
- All files created/modified
- All implementations
- All tests
- Edge cases covered?

## Required Assessments

1. **Code Quality**
   - Is the code production-ready?
   - Are there any code smells?
   - Is it maintainable?
   - Are there optimizations needed?

2. **Logic & Correctness**
   - Is the logic sound?
   - Are there edge cases not handled?
   - Are there potential bugs?
   - Is error handling comprehensive?

3. **Security**
   - Are there any security vulnerabilities?
   - Is input validation complete?
   - Are there injection risks?
   - Is sensitive data properly handled?

4. **Performance**
   - Are there performance bottlenecks?
   - Is it optimized?
   - Are there unnecessary operations?
   - Is database access efficient?

5. **Comprehensive Nature**
   - Are all edge cases covered?
   - Is error handling complete?
   - Are all user scenarios handled?
   - Is the implementation complete?

## Required Output

Provide:
1. **Critical Issues List** (must fix before proceeding)
2. **Improvements Needed** (should fix for excellence)
3. **Optimization Opportunities**
4. **Edge Cases Not Handled**
5. **Security Concerns**
6. **Performance Issues**
7. **Final Verdict**: PASS/FAIL with detailed reasoning

## Zero Tolerance Means

- No "good enough" - must be excellent
- No "it works" - must be robust
- No "later" - must be complete now
- No "maybe" - must be certain

Be brutal. Be thorough. Be excellent.
```

### Phase Review Workflow

1. **Verify Phase Complete:** Confirm all tasks in phase are `[x]`
2. **Collect Phase Commits:** List all commit hashes for the phase
3. **Invoke codex-reviewer:** Use the directive template above
4. **Review Findings:** Address ALL critical issues
5. **Re-test:** Ensure fixes do not break anything
6. **Document Review:** Create summary of review findings
7. **Update Phase Status:** Mark phase as "Reviewed & Approved"
8. **Only Then Proceed:** Move to next phase

### Failure Criteria

Phase review FAILS if any:
- Critical security vulnerabilities found
- Unhandled edge cases that could crash production
- Missing error handling for critical paths
- Performance issues that impact user experience
- Incomplete implementations
- Technical debt that blocks next phase

### Review Documentation

After review completion, create a review summary document:

**File:** `docs/phase-[X]-tzar-review.md`

**Content:**
- Phase reviewed
- Review date
- Reviewer (codex-reviewer)
- Critical issues found (if any)
- Improvements made
- Final verdict
- Approval to proceed to next phase

This document becomes part of the permanent project record.
