# Agency-Agents Into Maestro — Implementation Task List

**Created**: 2026-03-11
**Status**: In Progress

---

## Phase 1: Canonical Built-In Agent Definitions ← CURRENT

### Task 1.1: Create agent definition files [BLOCKING]
- [ ] `maestro/agents/validators/sentinel.md` — Evidence-Focused QA Agent
- [ ] `maestro/agents/reviewers/warden.md` — Skeptical Final Reviewer
- [ ] `maestro/agents/specialized/mender.md` — Remediation Coordinator
- [ ] `maestro/agents/planners/cartographer.md` — Discovery Researcher
- [ ] `maestro/agents/reviewers/prism.md` — UX Architecture Reviewer

### Task 1.2: Add registry entries [BLOCKED BY 1.1]
- [ ] Add all 5 agents to `maestro/agents/registry.yaml`
- [ ] Add new metadata fields: `fallback_for`, `requires_visual_evidence`, `external_backstops`, `install_targets`, `supports_graceful_fallback`
- [ ] Add keyword_mapping entries for new task types: `evidence-qa`, `final-review`, `remediate`, `discover`, `ux-review`

### Task 1.3: Extend selector with built-in-first logic [BLOCKED BY 1.2]
- [ ] Add new fields to `AgentDefinition` dataclass in `selector.py`: `fallback_for`, `requires_visual_evidence`, `external_backstops`, `supports_graceful_fallback`
- [ ] Add `select_with_fallback()` method to `AgentSelector` that prefers built-in → detects external → falls back
- [ ] Add external CLI availability detection utility

---

## Phase 2: Workflow and Fallback Wiring [BLOCKED BY Phase 1]

### Task 2.1: Add workflow presets to `workflows.rs` [BLOCKED BY 1.1]
- [ ] `implement-review-remediate` preset: kraken → critic → mender → kraken
- [ ] `implement-evidence-review` preset: kraken → sentinel → critic
- [ ] `ui-review-evidence-final` preset: kraken → prism → sentinel → warden
- [ ] `parallel-review-with-final-gate` preset: parallel critics → warden

### Task 2.2: Add new AgentRoles to mapping.rs [BLOCKED BY 2.1]
- [ ] Add `Sentinel`, `Warden`, `Mender`, `Cartographer`, `Prism` to `AgentRole` enum
- [ ] Add corresponding `PiAgentType` variants: `EvidenceValidator`, `FinalReviewer`, `Remediator`, `Researcher`, `UxReviewer`
- [ ] Add default mappings for new roles

### Task 2.3: Add bounded retry policy [BLOCKED BY 2.1]
- [ ] Add `RetryPolicy` struct to workflows.rs with `max_attempts`, `escalation_behavior`
- [ ] Wire retry policy into `WorkflowPreset`

---

## Phase 3: Handoff and Review Integration [BLOCKED BY Phase 2]

### Task 3.1: Extend handoff context keys
- [ ] Add `review_evidence`, `remediation_findings`, `escalation_reason`, `retry_count`, `review_verdict` to `VALID_CONTEXT_KEYS`

### Task 3.2: Add handoff templates for new flows
- [ ] QA evidence handoff template
- [ ] Remediation handoff template
- [ ] Escalation handoff template

---

## Phase 4: Docs and Verification [BLOCKED BY Phase 3]

### Task 4.1: Update docs
- [ ] Update `docs/AGENTS.md` with new built-in agents
- [ ] Update `docs/INSTALLATION.md` with fallback behavior
- [ ] No provenance leakage check

### Task 4.2: Tests
- [ ] Selector tests for built-in-first fallback
- [ ] Workflow preset tests for new presets
- [ ] Mapping tests for new roles
