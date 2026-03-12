# Agency-Agents Into Maestro

**Date**: 2026-03-11  
**Purpose**: Implementation-oriented analysis for absorbing high-value agent behaviors into Maestro  
**Primary source repo inspected**: `/tmp/agency-agents`  
**Maestro repo**: `/mnt/WD-SSD/Prod/maestro`

---

## Executive Summary

This document is intentionally **not** a system-vs-system comparison.

- `agency-agents` is an agent library plus operating discipline
- Maestro is a broader framework with installers, tracks, workflow rules, handoffs, memory, review UX, and Leindex-based analysis

The correct question is:

**Which parts of `agency-agents` should be re-implemented inside Maestro so they strengthen Maestro's own workflow, agent system, and tool integrations?**

### Core conclusion

The best value comes from re-implementing selected agent behaviors and workflow patterns as:

1. **Built-in Maestro agents**
2. **Installer-distributed tool-specific integrations**
3. **Optional external final-pass reviewers when their CLIs are available**

The imported functionality should be treated as a **clean Maestro-native implementation**, not as a visible port or branded transplant.

---

## Non-Negotiable Product Decisions

These decisions should drive the implementation.

### 1. Built-in agents must become the primary path

Today, Maestro can rely on external tool-specific agents such as Codex or Gemini-backed reviewers. That is valuable, but it is not universal because many users will not have those CLIs installed.

So the target design should be:

- Maestro-native built-in agents do the main planning, QA, review, and remediation work
- optional external agents are invoked only at the **highest-value checkpoint**, usually the final verification pass or a targeted expert pass
- if external agents are absent, Maestro continues without breaking and clearly informs the user that it is falling back to the built-in equivalent

### 2. Installer-selected tools decide where agent integrations are emitted

Maestro already lets users choose tool integrations during installation. The new built-in agents should be distributed into the tools the user selected during:

- installer script flow
- Conductor Wizard / TUI flow
- marketplace installation flow, where applicable

If a selected tool supports custom agents/prompts/commands, Maestro should install the Maestro-native agent material for that tool.

### 3. No visible provenance from agency-agents

Once re-implemented in Maestro:

- there should be no mention of `agency-agents`
- there should be no "ported from" wording
- agent prompts, names, docs, and installer output should read as first-party Maestro functionality

This is not only a branding choice. It also keeps the implementation clean and prevents long-term architectural confusion.

---

## Repo-Grounded Maestro Surfaces

The following existing Maestro seams should be treated as the actual integration targets.

### Workflow and policy

- `maestro/workflow.md`
- `maestro/maestro_code_styleguides/general.md`

These already define how Maestro expects work to happen, including review and quality gates.

### Agent registry and selection

- `maestro/agents/registry.yaml`
- `maestro/core/agents/selector.py`

These are the main places for first-class built-in agent definitions and selection policy.

### Handoffs and track continuity

- `maestro/memory/coordination/handoffs.py`
- `maestro/core/tracks/integrations.py`

These already support structured continuity and should absorb richer QA/remediation/escalation handoff patterns.

### Review UX

- `crates/cli/src/commands/tracklens.rs`
- `src/leindex/src/tracklens/types.rs`
- `crates/cockpit/src/tracklens/mod.rs`

TrackLens already provides allow/deny review semantics and is the right place for stronger built-in review personas and remediation loops.

### Workflow presets and orchestration

- `crates/pi-mono/src/agents/workflows.rs`

This is the right place to encode richer review chains and bounded retry/escalation behavior.

### Installer and tool selection

- `docs/INSTALLATION.md`
- `src/leindex/bin/setup_main.rs`
- `plugin.json`

Maestro already supports tool selection during install and documents first-class integration targets for:

- Claude Code
- OpenCode
- Codex CLI
- Gemini CLI
- Qwen Code
- Amp CLI
- Droid CLI

### Important implementation caution

`crates/pi-mono/src/detection.rs` currently describes capability detection as incomplete and default-based. Do not design the new agent rollout around a capability system that does not yet fully exist. Prefer explicit installer-time knowledge and conservative runtime fallbacks.

---

## Correct Architecture Direction

## Built-in first, external second

The correct hierarchy is:

1. **Built-in Maestro specialist agent executes**
2. **Built-in reviewer/validator checks**
3. **Optional external reviewer runs only if available and useful**
4. **If absent, built-in final QA agent completes the workflow**

This means external CLI agents should stop being the center of the design and become optional accelerators or final-pass auditors.

### Why this is the right shape

- it works for all users, not only users with Codex/Gemini/Qwen/etc installed
- it preserves Maestro's multi-tool philosophy
- it reduces external quota usage by invoking premium external models only after the work is already narrowed, refined, and review-ready
- it gives consistent behavior across tools even when the external ecosystem differs

---

## Recommended Built-In Agents To Implement

These should be re-implemented as Maestro-native agents with clean naming and first-party prompts.

## 1. Evidence-Focused QA Agent

### Purpose

A validator focused on:

- collecting concrete evidence
- requiring screenshots or observable proof for UI work
- documenting acceptance evidence
- rejecting vague "looks good" conclusions

### Best placement

- category: `validators` or `specialized`
- selection trigger: UI work, walkthrough review, acceptance verification

### Maestro role

This becomes the standard built-in fallback when no external final QA reviewer is available.

---

## 2. Skeptical Final Reviewer

### Purpose

A reviewer biased toward:

- defaulting to `needs work` until evidence is strong
- cross-checking claims against evidence
- rejecting incomplete or weakly-validated work

### Best placement

- category: `reviewers`
- used before task completion and before phase-close review

### Maestro role

This should be the built-in final gate that external reviewers complement rather than replace.

---

## 3. Remediation Coordinator

### Purpose

A workflow-oriented agent for:

- turning review findings into concrete fix tasks
- preserving context across retries
- escalating after bounded retry count

### Best placement

- category: `specialized` or `orchestrators`
- likely connected more through workflow presets than ad hoc selection

### Maestro role

This is the missing glue between review denial and productive retry.

---

## 4. Discovery Researcher

### Purpose

A planner/research agent for ambiguous early work:

- trend and product landscape research
- user feedback synthesis
- pre-spec discovery

### Best placement

- category: `planners` or `specialized`

### Maestro role

This is especially useful for greenfield tracks where implementation is not the hard part yet.

---

## 5. UX Architecture Reviewer

### Purpose

A design/interaction specialist for:

- UI structure
- frontend architecture
- flow coherence
- visual review readiness

### Best placement

- category: `reviewers` or `specialized`

### Maestro role

Useful for walkthrough review and design-heavy changes, especially before screenshot-based QA.

---

## What Should Stay External-Optional

These are excellent candidates for optional final-pass integrations:

- Codex final code review
- Gemini large-context review
- Qwen or OpenCode fast pattern checks
- other CLI-backed specialist auditors

### Rule

External tools should be used when they provide **incremental confidence**, not when they are required to make Maestro functional.

---

## Distribution Model Across Supported Tools

Maestro already documents that installation is tool-specific. The new built-in agents should follow the same model.

## Canonical source of truth

There should be one canonical Maestro-native agent definition per built-in agent in-repo, then tool-specific render/install steps should derive from that.

### Recommendation

Introduce a canonical internal representation, then render it to each tool format instead of duplicating agent logic independently per tool.

Possible structure:

- canonical prompts/specs in `maestro/agents/...`
- tool adapters/renderers in installer/setup code
- generated command/prompt files emitted into selected tool homes

### Why

- avoids prompt drift across tools
- makes updates cheap
- preserves one logical behavior with multiple tool transports

---

## Tool-specific rollout guidance

Grounded in `docs/INSTALLATION.md`, Maestro currently installs first-class integrations into the selected tools. Use that same installer source of truth for the new built-in agents.

### Claude Code

Install Maestro-native commands/templates/agent prompts into the Claude integration area already used by Maestro.

### Codex CLI

Install built-in prompt/agent material into `${CODEX_HOME:-~/.codex}/prompts/` and related config surfaces already managed by Maestro.

### Gemini CLI and Qwen

Install command/prompt material into their command directories and keep Leindex MCP integration intact.

### OpenCode

Install skill-compatible and command-compatible material into the existing Maestro OpenCode integration directories.

### Amp and Droid

Where full agent semantics are not symmetric with Claude/Codex/Gemini, still install the closest supported Maestro-native command/prompt integration instead of skipping feature parity entirely.

### Runtime behavior rule

At runtime, Maestro should know:

- which tools were selected at install time
- which of those support native agent/prompt installation
- which external CLIs are present right now

Use install-time configuration for the first two, runtime probing for the third.

---

## Proposed Runtime Selection Hierarchy

This is the recommended selection order for review and QA tasks.

## For standard implementation work

1. Maestro built-in planner/reviewer/validator chain
2. if configured and available, optional external final-pass reviewer
3. otherwise, built-in skeptical final reviewer completes the pass

## For UI-heavy work

1. built-in UX architecture review
2. built-in evidence-focused QA
3. optional external multimodal/final-pass review
4. fallback to built-in skeptical final reviewer

## For high-risk changes

1. built-in primary review
2. built-in remediation coordinator if denied
3. optional external final-pass auditor if available
4. bounded retries, then escalation

---

## Workflow Changes Recommended

## 1. Make review chains richer

Extend `crates/pi-mono/src/agents/workflows.rs` with presets like:

- `implement-review-remediate`
- `implement-evidence-review`
- `ui-review-evidence-final`
- `parallel-review-with-final-gate`

These should encode:

- the built-in reviewer sequence
- where optional external reviewers slot in
- bounded retry count
- escalation behavior

## 2. Add bounded retry policy

Add explicit retry policy to `maestro/workflow.md`:

- review fail -> remediation -> re-review
- max attempts: 3 by default
- on final failure, escalate with preserved context

This should be a Maestro policy, not an agent-local habit.

## 3. Add review-mode handoff templates

Extend handoff structures to support:

- QA evidence handoff
- remediation handoff
- escalation handoff
- final review handoff

This should enrich existing handoff infrastructure, not replace it.

---

## Registry and Selector Changes Recommended

## `maestro/agents/registry.yaml`

Add new built-in agents with metadata that supports:

- category
- selection triggers
- preferred task types
- whether visual evidence is required
- whether external backstop reviewers are compatible
- whether the agent is safe as a fallback for all tool environments

Suggested new metadata fields:

- `fallback_for`
- `requires_visual_evidence`
- `external_backstops`
- `install_targets`
- `supports_graceful_fallback`

## `maestro/core/agents/selector.py`

Extend selection logic so it can:

- prefer built-in agents first
- detect whether an optional external reviewer is available
- append the external reviewer only at the final stage
- fall back automatically without aborting the workflow

Do not make selector logic depend on brittle brand-specific assumptions. Model capability and role, not origin.

---

## Installer and Marketplace Changes Recommended

## Installer

Use installer tool-selection as the source of truth for where built-in agents are emitted.

Implementation areas to inspect/update:

- `src/leindex/bin/setup_main.rs`
- installer config persistence
- tool-specific install writers
- post-install verification

### Required installer behavior

When a user selects tools during install:

1. install Maestro-native built-in agents into each supported selected tool
2. detect optional external CLIs separately
3. if an external CLI is absent, do not fail installation
4. mark the external reviewer as unavailable but keep built-in fallback enabled

## Marketplace

Marketplace installs should also provision the built-in agent layer for supported tools, not only command wrappers.

This means the plugin/marketplace path should remain functionally useful even for users who never install external CLIs.

---

## Hard Rule: No Provenance Leakage

Once implemented:

- no agent file should mention `agency-agents`
- no user-facing docs should describe agents as imported or ported
- no runtime message should say "ported from"
- no installer output should mention source provenance

Internally, implementation notes can mention the source repo for engineering reference. User-facing behavior should remain purely Maestro-native.

---

## Implementation Plan

This is the recommended execution order for the implementation agent.

## Phase 1: Canonical built-in agent definitions

Create the first-party Maestro-native prompts/specs for:

- evidence QA
- skeptical final reviewer
- remediation coordinator
- discovery researcher
- UX architecture reviewer

Deliverables:

- new agent definition files
- registry entries
- selector support

## Phase 2: Workflow and fallback wiring

Add:

- richer workflow presets
- bounded retry and escalation behavior
- built-in-first, external-second selection logic

Deliverables:

- updated preset definitions
- updated workflow policy docs
- runtime fallback behavior

## Phase 3: Tool installation rollout

Teach installer/configure flows to:

- emit built-in agents into selected tools
- probe optional external CLIs
- register which external final-pass reviewers are available

Deliverables:

- installer changes
- configure updates
- per-tool output verification

## Phase 4: Handoff and review UX integration

Integrate:

- new handoff templates
- TrackLens alignment for deny/remediate/review loops
- clearer review artifacts

Deliverables:

- handoff schema/template additions
- TrackLens review flow enhancements

## Phase 5: Docs and verification

Update:

- `docs/INSTALLATION.md`
- tool-specific docs such as `docs/CODEX.md` and `docs/GEMINI.md`
- any configure/help surfaces

Deliverables:

- docs for built-in fallback behavior
- docs for optional external final-pass reviewers

---

## Recommended Coding and Operational Guidance

The next implementation agent should follow these rules.

## Design rules

- Keep one canonical built-in agent definition per logical agent and render tool-specific variants from it.
- Do not duplicate prompt logic across six tools manually.
- Model capability and role, not hardcoded vendor identity, where possible.
- External reviewer absence must be non-fatal.
- Fallback behavior must be explicit and user-visible.
- Preserve Maestro's existing workflow spine instead of layering a second orchestration system beside it.

## Code style rules

Follow:

- `maestro/workflow.md`
- `maestro/maestro_code_styleguides/general.md`
- applicable language-specific style guides in `maestro/maestro_code_styleguides/`

Especially enforce these principles from the general guide:

- obvious code over clever code
- boundary validation first
- separate pure logic from IO
- no swallowed errors or blind retries
- small, testable changes over broad rewrites

## Prompt/agent writing rules

- write the new agents as if they were always native to Maestro
- remove all provenance references
- keep prompts role-specific and outcome-specific
- ensure each agent has a narrow, testable responsibility
- make remediation agents preserve context, not rewrite history

## Runtime behavior rules

- missing external CLIs must degrade gracefully
- user messaging must explain that Maestro is using the built-in reviewer instead
- do not hard fail just because an external reviewer is unavailable
- invoke expensive external final-pass reviewers only after built-in review has narrowed the work

---

## Test Strategy Recommended

The implementation should include tests for:

## Selector behavior

- built-in reviewer chosen when no external CLI exists
- external final-pass reviewer appended when available
- correct fallback for UI-heavy tasks

## Installer behavior

- selected tools receive built-in agent artifacts
- unselected tools receive nothing
- missing external CLIs do not fail install
- post-install config reflects available reviewers correctly

## Workflow behavior

- review fail -> remediation -> retry loop works
- escalation occurs after max retry count
- handoff context preserves review findings across retries

## Docs and UX behavior

- user-facing docs describe built-in-first behavior accurately
- no user-facing text leaks provenance

---

## What Not To Do

- Do not treat external CLI-backed reviewers as the default implementation path.
- Do not create separate per-tool copies of the same logical agent unless generation/rendering is impossible.
- Do not replace Maestro tracks, TrackLens, or handoff infrastructure.
- Do not add user-facing references to `agency-agents`.
- Do not block installer success on external reviewer availability.

---

## Final Recommendation

Maestro should absorb the best parts of `agency-agents` by re-implementing them as:

- Maestro-native built-in agents
- installer-distributed integrations for the user-selected tools
- optional external final-pass reviewers layered on top

That gives Maestro:

- much stronger default QA and review behavior
- first-class operation even without external CLIs
- better cross-tool consistency
- cleaner product identity
- lower-cost use of premium external reviewers

The key architectural principle is simple:

**Built-in agents do the work. External agents, when present, provide the last high-value audit pass.**
