# Track: {{TRACK_ID}}

## Track Metadata
- **Created:** {{TIMESTAMP}}
- **Status:** ⬜ Not Started
- **Priority:** {{PRIORITY}}
- **Estimated Complexity:** {{COMPLEXITY}}
- **Agent Tool:** {{AGENT_TOOL}}

---

## Overall Goal
{{GOAL_DESCRIPTION}}

### Success Criteria
1. {{CRITERION_1}}
2. {{CRITERION_2}}

### Sub-Goals
- {{SUBGOAL_1}}
- {{SUBGOAL_2}}

### Scope Boundaries
- **IN SCOPE:** {{IN_SCOPE}}
- **OUT OF SCOPE:** {{OUT_OF_SCOPE}}
- **ALLOWED TANGENTS:** {{TANGENTS}}

---

## Phase 1: {{PHASE_NAME}}

### Status: ⬜ Not Started

### Tasks

#### Task 1.1: Write Failing Tests for {{FEATURE}} ⬜
**Type:** TEST-FIRST (BLOCKING — must complete before Task 1.2)
**Description:** Create comprehensive tests that verify {{FEATURE}} works correctly.

**Test Requirements:**
- Tests MUST fail until proper implementation is provided
- Tests MUST NOT be passable by mock/stub/simplified implementations
- Tests MUST cover:
  - WHAT: Expected functionality and behavior
  - HOW: Implementation quality and correctness
  - WHY: Purpose alignment with spec

**Acceptance Criteria:**
- [ ] All tests written and failing with clear error messages
- [ ] Tests cover happy path, edge cases, and error cases
- [ ] No test can pass without genuine implementation

#### Task 1.2: Implement {{FEATURE}} ⬜
**Type:** IMPLEMENTATION (BLOCKED BY: Task 1.1)
**Description:** Implement the feature to make all Task 1.1 tests pass.

**Acceptance Criteria:**
- [ ] All Task 1.1 tests pass
- [ ] No mocks, stubs, or simplified implementations
- [ ] Code follows project style guide
- [ ] Code reviewed

#### Task 1.3: Phase Review & Verification ⬜
**Type:** REVIEW (BLOCKED BY: Task 1.2)
**Description:** Review all work in Phase 1.

**Acceptance Criteria:**
- [ ] All tests pass
- [ ] Code quality verified
- [ ] LLM Remarks section filled out

### LLM Remarks — Phase 1
> **Work Summary:** {{TO_BE_FILLED}}
> **Learnings:** {{TO_BE_FILLED}}
> **Issues Encountered:** {{TO_BE_FILLED}}
> **Decisions Made:** {{TO_BE_FILLED}}
> **Logical Throughline:** {{TO_BE_FILLED}}
> **Justifications:** {{TO_BE_FILLED}}
> **Self-Evaluation:** {{TO_BE_FILLED}}

---

## Phase N: {{PHASE_NAME}}
<!-- Repeat the Phase 1 structure for each additional phase -->

---

## Track Progress Summary
| Phase | Status | Tasks Done | Tests Passing |
|-------|--------|------------|---------------|
| Phase 1 | ⬜ | 0/3 | 0/0 |

## Track Completion Checklist
- [ ] All phases completed
- [ ] All tests passing
- [ ] All LLM Remarks sections filled
- [ ] Final self-evaluation written
- [ ] Code reviewed
