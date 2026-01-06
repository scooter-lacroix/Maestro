# Project Workflow (OpenCode)

## Guiding Principles

1. **The Plan is the Source of Truth:** All work must be tracked in `plan.md`
2. **The Tech Stack is Deliberate:** Changes to the tech stack must be documented in `tech-stack.md` *before* implementation
3. **Test-Driven Development:** Write unit tests before implementing functionality
4. **High Code Coverage:** Aim for >98% code coverage for all modules
5. **User Experience First:** Every decision should prioritize user experience
6. **Non-Interactive & CI-Aware:** Prefer non-interactive commands. Use `CI=true` for watch-mode tools (tests, linters) to ensure single execution.
7. **Critical Think First:** Apply systematic metacognitive analysis before and after key actions to ensure quality and prevent common pitfalls.

## OpenCode Agent Integration

**IMPORTANT:** This workflow is configured for OpenCode's agent ecosystem. Agent names below match the actual OpenCode agent configurations.

### OpenCode Agent Mapping

| Agent Name | Specialty | When Used |
|------------|-----------|-----------|
| **codex-reviewer** | Architecture, code review, strategy | MANDATORY for all implementation reviews |
| **gemini-analyzer** | Multi-repo analysis, doc lookup, large codebase exploration | Large codebase (>100KB), research tasks |
| **opencode-scaffolder** | Fast codebase exploration, pattern matching, API scaffolding | Standard implementation (5-50 lines), rapid prototyping |
| **qwen-coder** | Refactoring, test generation, SOLID principles | Code polish, comprehensive testing |
| **amp-code** | ETL/ELT data pipelines, multi-stage data workflows | Data engineering, validation, enrichment |
| **rovo-dev** | Complex multi-file orchestration, documentation-driven development | Complex tasks (>50 lines), full-featured implementations |
| **opus-specialist** | Complex algorithms, multi-layered logic reasoning | Tasks requiring deep reasoning and edge case consideration |
| **gemini-frontend-designer** | UI/UX design and implementation | Frontend tasks, UI components |
| **sonnet-specialist** | Technical writing, practical implementation | Documentation, day-to-day development |
| **general-purpose** | General tasks, research | Versatile tasks without specific requirements |

### Orchestrator Agents

- **kilocode-orchestrator**: Large-scale projects with persistent memory across sessions
- **llm-council-evaluator**: Meta-agent selection for high-risk or complex decisions
- **droid-factory**: Spec-driven development with automatic agent selection

## Agent Usage Requirements

**CRITICAL SYSTEM DIRECTIVE: PROACTIVE AUTOMATIC AGENT USAGE**

1. **Automatic Agent Selection is MANDATORY:** Agent usage is the DEFAULT behavior, NOT an optional enhancement. You MUST proactively select and launch agents based on task complexity WITHOUT waiting for user instruction.

2. **NEVER Ask for Agent Permission:** Do NOT ask "Should I use an agent?" or "Which agent should I use?". Make the decision automatically based on task complexity and launch the appropriate agents.

3. **Agent Selection Criteria (Execute Automatically):**
   - **Trivial tasks (1-5 lines, simple changes):** Implement directly
   - **Standard tasks (5-50 lines, single file):** Automatically use opencode-scaffolder
   - **Complex tasks (multiple files, >50 lines):** Automatically use codex-reviewer or gemini-analyzer for design + opencode-scaffolder for implementation
   - **Large codebase analysis (>100KB):** Automatically use gemini-analyzer
   - **ETL/Data pipelines:** Automatically use amp-code
   - **Spec-driven/ambiguous requirements:** Automatically use droid-factory
   - **ALL implementation work:** MUST be automatically followed by codex-reviewer for validation

4. **Mandatory Pre-Commit Review:** Before marking any task complete and committing:
   - ALL code changes MUST be automatically reviewed by codex-reviewer
   - Review results MUST be addressed before proceeding
   - If critical issues are found, they MUST be fixed before commit

## Agent Availability and Fallbacks

### Checking Tool Availability

Before using specialized agents that require external CLI tools, you MUST check if the required tools are available on the user's system.

**Check Command:**
```bash
# Check if perspective CLI tools are available
which gemini 2>/dev/null && which qwen 2>/dev/null && which codex 2>/dev/null
```

### Graceful Degradation

If an external agent is unavailable but recommended:
1. Inform the user which agent is unavailable
2. Provide the recommended alternative
3. Ask if they want to proceed with the alternative
4. Continue work with user's approval

### Tool-Specific Requirements

- **gemini-analyzer**: Requires `gemini` CLI with API keys configured
- **opencode-scaffolder**: Built-in (no external tools required)
- **qwen-coder**: Requires `qwen` CLI with API keys configured
- **codex-reviewer**: Requires `codex` CLI with API keys configured
- **amp-code**: Built-in (no external tools required)
- **rovo-dev**: Built-in (no external tools required)

**NOTE:** If external CLI tools are not available, OpenCode will automatically fall back to using built-in agent capabilities. Users can configure external tools by running `/maestro:configure`.

## Task Workflow

All tasks follow a strict lifecycle:

### Standard Task Workflow

**AUTOMATIC AGENT SELECTION:**
- You MUST assess task complexity and launch appropriate agents automatically
- Do NOT await user instruction - agent usage is automatic and proactive

1. **Select Task:** Choose the next available task from `plan.md` in sequential order

2. **Mark In Progress:** Before beginning work, edit `plan.md` and change the task from `[ ]` to `[~]`

3. **Assess Complexity and Select Agent (AUTOMATIC):**
   - **CRITICAL:** Assess task complexity and automatically select the appropriate approach:
     - Trivial tasks (1-5 lines): Implement directly
     - Standard tasks (5-50 lines, single file): Automatically launch opencode-scaffolder
     - Complex tasks (multiple files, >50 lines): Automatically launch codex-reviewer or gemini-analyzer for design + opencode-scaffolder for implementation
     - ETL/Data tasks: Automatically launch amp-code
   - **Do NOT ask user permission** - this is automatic

4. **Write Failing Tests (Red Phase):**
   - Create a new test file for the feature or bug fix
   - Write one or more unit tests that clearly define the expected behavior
   - **CRITICAL:** Run the tests and confirm that they fail as expected
   - **AUTOMATIC AGENT:** For test writing, automatically use qwen-coder or opencode-scaffolder

5. **Implement to Pass Tests (Green Phase):**
   - Write the minimum amount of application code necessary to make tests pass
   - Run the test suite again and confirm all tests pass
   - **AUTOMATIC AGENT:** For implementation, automatically use opencode-scaffolder or rovo-dev (complex)

6. **Refactor (Optional but Recommended):**
   - With passing tests, refactor to improve clarity, remove duplication, enhance performance
   - Rerun tests to ensure they still pass
   - **AUTOMATIC AGENT:** Automatically use qwen-coder for refactoring

7. **Verify Coverage:** Run coverage reports. Target: >98% coverage for new code.

8. **Document Deviations:** If implementation differs from tech stack:
   - **STOP** implementation
   - Update `tech-stack.md` with new design
   - Add dated note explaining the change
   - Resume implementation

9. **Agent Review (MANDATORY - AUTOMATIC):**
   - **CRITICAL:** Before proceeding to commit, you MUST automatically launch codex-reviewer
   - **AUTOMATICALLY Launch Code Review:** Use codex-reviewer to review all changes
   - **Address Review Findings:** Fix critical issues before proceeding
   - **Confirm Review Complete:** Only after codex-reviewer passes should you proceed

10. **Commit Code Changes:**
    - Stage all code changes related to the task
    - Propose a clear, concise commit message
    - Perform the commit

11. **Create Obsidian Note for Task Summary:**
    - **Step 11.1: Get Commit Hash:** `git log -1 --format="%H"`
    - **Step 11.2: Draft Note Content:** Create detailed summary including task name, changes, files, rationale
    - **Step 11.3: Create Obsidian Note:** Write to configured vault
      ```bash
      OBSIDIAN_VAULT="${OBSIDIAN_VAULT_PATH:-$HOME/Documents/ObsidianVault}"
      NOTE_PATH="$OBSIDIAN_VAULT/Tasks/$(date +%Y%m%d)-task-summary.md"
      mkdir -p "$(dirname "$NOTE_PATH")"
      # Create note with YAML frontmatter and content
      ```

12. **Get and Record Task Commit SHA:**
    - **Step 12.1: Update Plan:** Read `plan.md`, update task from `[~]` to `[x]`, append commit hash
    - **Step 12.2: Write Plan:** Write updated content back to `plan.md`

13. **Commit Plan Update:**
    - Stage the modified `plan.md` file
    - Commit with descriptive message (e.g., `maestro(plan): Mark task complete`)

---

## Phase Completion: "Tzar of Excellence" Review

**MANDATORY REQUIREMENT: At the completion of EACH phase, a rigorous zero-tolerance code review MUST be conducted.**

### When to Trigger

After ALL tasks in a phase are marked complete `[x]` in `plan.md`, BEFORE moving to the next phase:

1. **Deploy codex-reviewer with "Tzar of Excellence" directive**
2. **Wait for review completion**
3. **Address all critical findings**
4. **Only then proceed to next phase**

### "Tzar of Excellence" Directive Template

```
You are conducting the "Tzar of Excellence" review for Phase [X].

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

1. **Code Quality** - Production-ready? Maintainable? Optimized?
2. **Logic & Correctness** - Sound logic? Edge cases? Error handling?
3. **Security** - Vulnerabilities? Input validation? Injection risks?
4. **Performance** - Bottlenecks? Optimizations? Unnecessary operations?
5. **Comprehensive Nature** - Edge cases covered? Error handling complete? Implementation complete?

## Required Output

Provide:
1. **Critical Issues List** (must fix before proceeding)
2. **Improvements Needed** (should fix for excellence)
3. **Optimization Opportunities**
4. **Edge Cases Not Handled**
5. **Security Concerns**
6. **Performance Issues**
7. **Final Verdict**: PASS/FAIL with detailed reasoning

Be brutal. Be thorough. Be excellent.
```

### Phase Review Workflow

1. **Verify Phase Complete:** Confirm all tasks in phase are `[x]`
2. **Collect Phase Commits:** List all commit hashes for the phase
3. **Invoke codex-reviewer:** Use the directive template above
4. **Review Findings:** Address ALL critical issues
5. **Re-test:** Ensure fixes don't break anything
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

After review completion, create: `docs/phase-[X]-tzar-review.md`

Content includes:
- Phase reviewed
- Review date
- Reviewer (codex-reviewer)
- Critical issues found
- Improvements made
- Final verdict
- Approval to proceed to next phase

---

## Quality Gates Checklist

Before marking any task or phase complete, verify:

- [ ] All tests pass
- [ ] 98%+ code coverage
- [ ] Code style compliance
- [ ] Documentation completeness
- [ ] Type safety enforcement
- [ ] No linting/static analysis errors
- [ ] Updated documentation
- [ ] No security vulnerabilities

## Development Commands

**AI AGENT INSTRUCTION: Adapt this section to the project's specific language, framework, and build tools.**

### Setup
```bash
# Example: Commands to set up the development environment
# e.g., npm install, go mod tidy, pip install -r requirements.txt
```

### Daily Development
```bash
# Example: Commands for common daily tasks
# e.g., npm run dev, npm test, npm run lint
```

### Before Committing
```bash
# Example: Commands to run all pre-commit checks
# e.g., npm run check, make check
```

## Testing Requirements

### Unit Testing
- Every module must have corresponding tests
- Use appropriate test setup/teardown mechanisms
- Mock external dependencies
- Test both success and failure cases

### Integration Testing
- Test complete user flows
- Verify database transactions
- Test authentication and authorization
- Check form submissions

## Code Review Process

### Self-Review Checklist

Before requesting review:

1. **Functionality**
   - Feature works as specified
   - Edge cases handled
   - Error messages are user-friendly

2. **Code Quality**
   - Follows style guide
   - DRY principle applied
   - Clear variable/function names
   - Appropriate comments

3. **Testing**
   - Unit tests comprehensive
   - Integration tests pass
   - Coverage adequate (>98%)

4. **Security**
   - No hardcoded secrets
   - Input validation present
   - SQL injection prevented
   - XSS protection in place

5. **Performance**
   - Database queries optimized
   - Images optimized
   - Caching implemented where needed

## Commit Guidelines

### Message Format
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests
- `chore`: Maintenance tasks

### Examples
```bash
git commit -m "feat(auth): Add remember me functionality"
git commit -m "fix(posts): Correct excerpt generation for short posts"
git commit -m "test(comments): Add tests for emoji reaction limits"
```

## Definition of Done

A task is complete when:

1. All code implemented to specification
2. Unit tests written and passing
3. Code coverage meets project requirements (>98%)
4. Documentation complete (if applicable)
5. Code passes all configured linting and static analysis checks
6. Code reviewed by codex-reviewer with all critical issues addressed
7. Implementation notes added to `plan.md`
8. Changes committed with proper message
9. Obsidian note with task summary created

## Emergency Procedures

### Critical Bug in Production
1. Create hotfix branch from main
2. Write failing test for bug
3. Implement minimal fix
4. Test thoroughly
5. Deploy immediately
6. Document in plan.md

### Data Loss
1. Stop all write operations
2. Restore from latest backup
3. Verify data integrity
4. Document incident
5. Update backup procedures

### Security Breach
1. Rotate all secrets immediately
2. Review access logs
3. Patch vulnerability
4. Notify affected users (if any)
5. Document and update security procedures

## Deployment Workflow

### Pre-Deployment Checklist
- [ ] All tests passing
- [ ] Coverage >98%
- [ ] No linting errors
- [ ] Environment variables configured
- [ ] Database migrations ready
- [ ] Backup created

### Deployment Steps
1. Merge feature branch to main
2. Tag release with version
3. Push to deployment service
4. Run database migrations
5. Verify deployment
6. Test critical paths
7. Monitor for errors

### Post-Deployment
1. Monitor analytics
2. Check error logs
3. Gather user feedback
4. Plan next iteration

## Continuous Improvement

- Review workflow weekly
- Update based on pain points
- Document lessons learned
- Optimize for user happiness
- Keep things simple and maintainable

## OpenCode-Specific Notes

### Agent Configuration

To configure external CLI tools for enhanced agent capabilities, run `/maestro:configure`. This will:
1. Check for available CLI tools (gemini, qwen, codex)
2. Create appropriate agent configurations
3. Set up API key requirements
4. Verify tool functionality

### Memory System

OpenCode uses `memori-memory-mcp` for project context storage and retrieval. Ensure this MCP server is configured and running for full Maestro functionality.

### TUI Support

**NOTE:** TUI support is planned for a future release (v1.1). Current version focuses on CLI-based workflows.

### Memory Dashboard

**NOTE:** Memory Dashboard integration is planned for a future release (v1.1). Current version uses command-based status reporting via `/maestro:status`.
