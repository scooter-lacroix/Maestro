# OpenCode Maestro Implementation Dependencies

**Purpose**: Visual dependency map for implementation phases
**Format**: Text-based dependency graph

## Phase Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         OPENCODE MAESTRO IMPLEMENTATION                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 1: FOUNDATION                                                        │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐               │
│  │  1.1 Create  │────▶│  1.2 Create  │────▶│  1.3 Create  │               │
│  │  Commands    │     │  Workflow    │     │  Styleguides │               │
│  │  (CRITICAL)  │     │  (CRITICAL)  │     │  (HIGH)      │               │
│  └──────────────┘     └──────────────┘     └──────────────┘               │
│         │                                       │                           │
│         └───────────────────┬───────────────────┘                           │
│                             ▼                                               │
│  ┌───────────────────────────────────────────────────────────┐             │
│  │               PARALLEL OPPORTUNITY POINT 1                │             │
│  │  • 1.3 Styleguides can run parallel to 2.1 Config        │             │
│  └───────────────────────────────────────────────────────────┘             │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  PHASE 2: CONFIGURATION                                                     │
│  ┌──────────────┐     ┌──────────────┐                                    │
│  │  2.1 Create  │────▶│  2.2 Create  │                                    │
│  │  OpenCode    │     │  Installer   │                                    │
│  │  Config      │     │  Script      │                                    │
│  │  (CRITICAL)  │     │  (HIGH)      │                                    │
│  └──────────────┘     └──────────────┘                                    │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                         │
│  │  BLOCKS      │                                                         │
│  │  Phase 3     │─────────────────────────────────────────────────────┐   │
│  └──────────────┘                                                         │   │
│                                                                            │   │
├────────────────────────────────────────────────────────────────────────────┤
│  PHASE 3: COMMAND MODIFICATIONS                                            │
│  ┌──────────────┐                                                         │   │
│  │  3.1 Modify  │◀────── REQUIRED ──────┐                                 │   │
│  │  setup.md    │                       │                                 │   │
│  │  (HIGH)      │                       │                                 │   │
│  └──────────────┘                       │                                 │   │
│         │                                │                                 │   │
│         ▼                                │                                 │   │
│  ┌──────────────┐     ┌──────────────┐  │                                 │   │
│  │  3.2 Modify  │     │  3.3 Modify  │  │                                 │   │
│  │  newTrack.md │     │  implement.md│  │                                 │   │
│  │  (HIGH)      │     │  (HIGH)      │  │                                 │   │
│  └──────────────┘     └──────────────┘  │                                 │   │
│         │                    │           │                                 │   │
│         └──────────┬─────────┘           │                                 │   │
│                    ▼                     │                                 │   │
│            ┌──────────────┐             │                                 │   │
│            │  3.4 Modify  │             │                                 │   │
│            │  Remaining   │             │                                 │   │
│            │  Commands    │             │                                 │   │
│            │  (MEDIUM)    │             │                                 │   │
│            └──────────────┘             │                                 │   │
│                    │                     │                                 │   │
│                    └─────────────────────┘                                 │   │
│                                                                          │   │
│  ┌───────────────────────────────────────────────────────────┐         │   │
│  │               PARALLEL OPPORTUNITY POINT 2                │         │   │
│  │  • 3.2, 3.3, 3.4 can run in parallel after 3.1 complete  │         │   │
│  └───────────────────────────────────────────────────────────┘         │   │
│                                                                            │   │
├────────────────────────────────────────────────────────────────────────────┤
│  PHASE 4: SCRIPTS & TEMPLATES                                              │
│  ┌──────────────┐                                                         │   │
│  │  4.1 Update  │◀────── REQUIRED ──────┐                                 │   │
│  │  Scripts     │                       │                                 │   │
│  │  (MEDIUM)    │                       │                                 │   │
│  └──────────────┘                       │                                 │   │
│         │                                │                                 │   │
│         ▼                                │                                 │   │
│  ┌──────────────┐                       │                                 │   │
│  │  4.2 Create  │                       │                                 │   │
│  │  OpenCode    │                       │                                 │   │
│  │  Templates   │                       │                                 │   │
│  │  (LOW)       │                       │                                 │   │
│  └──────────────┘                       │                                 │   │
│         │                                │                                 │   │
│         └────────────────────────────────┘                                 │   │
│                                                                            │   │
├────────────────────────────────────────────────────────────────────────────┤
│  PHASE 5: DOCUMENTATION                                                     │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐             │   │
│  │  5.1 Update  │────▶│  5.2 Update  │────▶│  5.3 Update  │             │   │
│  │  SKILL.md    │     │  README.md   │     │  OPENCODE.md │             │   │
│  │  (HIGH)      │     │  (HIGH)      │     │  (HIGH)      │             │   │
│  └──────────────┘     └──────────────┘     └──────────────┘             │   │
│         │                    │                   │                         │   │
│         └────────────────────┴───────────────────┘                         │   │
│                              │                                               │   │
│                              ▼                                               │   │
│                     ┌──────────────┐                                        │   │
│                     │  5.4 Create  │                                        │   │
│                     │  Migration   │                                        │   │
│                     │  Guide       │                                        │   │
│                     │  (MEDIUM)    │                                        │   │
│                     └──────────────┘                                        │   │
│                                                                            │   │
│  ┌───────────────────────────────────────────────────────────┐           │   │
│  │               PARALLEL OPPORTUNITY POINT 3                │           │   │
│  │  • 5.1, 5.2, 5.3 can run in parallel after Phase 4       │           │   │
│  └───────────────────────────────────────────────────────────┘           │   │
│                                                                            │   │
├────────────────────────────────────────────────────────────────────────────┤
│  PHASE 6: TESTING                                                           │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐             │   │
│  │  6.1 Unit    │     │  6.2         │     │  6.3 Docs    │             │   │
│  │  Testing     │     │  Integration │     │  Testing     │             │   │
│  │  (CRITICAL)  │     │  (HIGH)      │     │  (MEDIUM)    │             │   │
│  └──────────────┘     └──────────────┘     └──────────────┘             │   │
│         │                    │                   │                         │   │
│         └────────────────────┴───────────────────┘                         │   │
│                              │                                               │   │
│                              ▼                                               │   │
│                     ┌──────────────┐                                        │   │
│                     │  ALL TESTS   │                                        │   │
│                     │  PASS        │                                        │   │
│                     │              │                                        │   │
│                     └──────────────┘                                        │   │
│                                                                            │   │
│  ┌───────────────────────────────────────────────────────────┐           │   │
│  │               PARALLEL OPPORTUNITY POINT 4                │           │   │
│  │  • 6.1, 6.2, 6.3 can run in parallel after Phase 5       │           │   │
│  └───────────────────────────────────────────────────────────┘           │   │
│                                                                            │   │
├────────────────────────────────────────────────────────────────────────────┤
│  PHASE 7: RELEASE                                                           │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐             │   │
│  │  7.1 Create  │────▶│  7.2 Prepare │────▶│  7.3 Tag &   │             │   │
│  │  Checklist   │     │  Release     │     │  Release     │             │   │
│  │  (HIGH)      │     │  Notes       │     │  (HIGH)      │             │   │
│  └──────────────┘     └──────────────┘     └──────────────┘             │   │
│                                                                            │   │
│                              ▲                                               │   │
│                              └───────────────────────────────────────────────┘   │
│                                    REQUIRED FROM ALL PREVIOUS PHASES            │
│                                                                            │   │
└────────────────────────────────────────────────────────────────────────────┘

LEGEND:
  ─────▶  Sequential dependency (must complete first)
  ─────│  Parallel opportunity (can run simultaneously)
  ⬢     Critical path items (must follow this order)
```

## Critical Path Analysis

### MUST BE SEQUENTIAL (No Parallelism)
```
Phase 1.1 (Commands) → Phase 1.2 (Workflow) → Phase 3.1 (setup.md)
                                                         ↓
Phase 1.3 + Phase 2.1 → Phase 2.2 → Phase 3 (all) → Phase 4 → Phase 5 → Phase 6 → Phase 7
```

### CAN BE PARALLEL (Opportunities)
```
Point 1: Phase 1.3 (Styleguides) ║ Phase 2.1 (Config)
Point 2: Phase 3.2, 3.3, 3.4 (after 3.1)
Point 3: Phase 5.1, 5.2, 5.3 (after Phase 4)
Point 4: Phase 6.1, 6.2, 6.3 (after Phase 5)
```

## Risk Dependencies

### HIGH RISK Dependencies (If these fail, downstream blocked)
1. **Phase 1.1 → Phase 3**: Without command files, cannot modify them
2. **Phase 1.2 → Phase 3**: Without workflow, agent selection broken
3. **Phase 2.1 → Phase 3**: Without config, commands can't reference agents
4. **Phase 3.3 → Phase 6**: implement.md changes critical for testing

### MEDIUM RISK Dependencies
1. **Phase 1.3 → Phase 5**: Styleguides needed for documentation
2. **Phase 2.2 → Phase 6**: Installer needed for integration testing
3. **Phase 4 → Phase 5**: Scripts needed for documentation examples

### LOW RISK Dependencies
1. **Phase 5.4 → Phase 7**: Migration guide nice-to-have, not blocking
2. **Phase 4.2 → Phase 6**: OpenCode templates optional

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DATA FLOW                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  SOURCE FILES                     TARGET FILES                              │
│  ┌─────────────┐                 ┌─────────────┐                            │
│  │ claude-code/│                 │ opencode/   │                            │
│  │  commands/  ├────COPY────────▶│ skill/      │                            │
│  │             │                 │ maestro/    │                            │
│  │  *.md       │                 │ commands/   │                            │
│  └─────────────┘                 └─────────────┘                            │
│         │                                                                 │
│         │ COPY                                                          │
│         ▼                                                                 │
│  ┌─────────────┐                 ┌─────────────┐                            │
│  │ claude-code/│                 │ opencode/   │                            │
│  │ templates/  ├────COPY────────▶│ skill/      │                            │
│  │             │                 │ maestro/    │                            │
│  │ workflow.md │                 │ templates/  │                            │
│  └─────────────┘                 └─────────────┘                            │
│         │                                                                 │
│         │ COPY                                                          │
│         ▼                                                                 │
│  ┌─────────────┐                 ┌─────────────┐                            │
│  │ claude-code/│                 │ opencode/   │                            │
│  │ templates/  ├────COPY────────▶│ skill/      │                            │
│  │ code_       │                 │ maestro/    │                            │
│  │ styleguides/│                 │ templates/  │                            │
│  └─────────────┘                 │ code_       │                            │
│                                  │ styleguides/│                            │
│                                  └─────────────┘                            │
│                                                                            │
│  MODIFICATION FLOW                                                         │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                  │
│  │ Agent       │     │ Command     │     │ Template    │                  │
│  │ Mappings    │────▶│ File Edits  │────▶│ Path Edits  │                  │
│  │ (codex-     │     │ (manual/    │     │ (sed/       │                  │
│  │  reviewer)  │     │  automated) │     │  manual)    │                  │
│  └─────────────┘     └─────────────┘     └─────────────┘                  │
│         │                                                                 │
│         │ VALIDATE                                                       │
│         ▼                                                                 │
│  ┌─────────────┐                 ┌─────────────┐                            │
│  │ Testing     │                 │ OpenCode    │                            │
│  │ &           ├────────────────▶│ Integration │                            │
│  │ Verification│                 │ Complete    │                            │
│  └─────────────┘                 └─────────────┘                            │
│                                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

## File Transformation Matrix

| Source File | Target File | Transformation Type | Priority |
|-------------|-------------|---------------------|----------|
| `/claude-code/commands/maestro:setup.md` | `/opencode/skill/maestro/commands/maestro:setup.md` | Copy + Modify | HIGH |
| `/claude-code/commands/maestro:newTrack.md` | `/opencode/skill/maestro/commands/maestro:newTrack.md` | Copy + Modify | HIGH |
| `/claude-code/commands/maestro:implement.md` | `/opencode/skill/maestro/commands/maestro:implement.md` | Copy + Modify | HIGH |
| `/claude-code/commands/maestro:status.md` | `/opencode/skill/maestro/commands/maestro:status.md` | Copy + Modify | MEDIUM |
| `/claude-code/commands/maestro:revert.md` | `/opencode/skill/maestro/commands/maestro:revert.md` | Copy + Modify | MEDIUM |
| `/claude-code/commands/maestro:configure.md` | `/opencode/skill/maestro/commands/maestro:configure.md` | Copy + Modify | MEDIUM |
| `/claude-code/commands/maestro:tui.md` | `/opencode/skill/maestro/commands/maestro:tui.md` | Copy + Modify | LOW |
| `/claude-code/commands/maestro:memory.md` | `/opencode/skill/maestro/commands/maestro:memory.md` | Copy + Modify | LOW |
| `/claude-code/templates/workflow.md` | `/opencode/skill/maestro/templates/workflow.md` | Copy + Heavy Modify | CRITICAL |
| `/claude-code/templates/code_styleguides/*` | `/opencode/skill/maestro/templates/code_styleguides/*` | Copy | HIGH |

## Testing Dependency Tree

```
                    ┌─────────────────┐
                    │   All Phases    │
                    │   Complete      │
                    └────────┬────────┘
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
     ┌──────────────┐                 ┌──────────────┐
     │  Unit Tests  │                 │  Integration │
     │  (6.1)       │                 │  Tests (6.2) │
     └──────┬───────┘                 └──────┬───────┘
            │                                │
            └────────────┬───────────────────┘
                         ▼
                  ┌──────────────┐
                  │  Doc Tests   │
                  │  (6.3)       │
                  └──────┬───────┘
                         │
                         ▼
                  ┌──────────────┐
                  │  All Pass    │
                  │  ✓           │
                  └──────────────┘
                         │
                         ▼
                  ┌──────────────┐
                  │  Ready for   │
                  │  Release     │
                  └──────────────┘
```

## Blocking Scenarios

### Scenario 1: Agent Configuration Not Ready
```
BLOCKS: Phase 3.3 (maestro:implement.md)
REASON: Cannot test agent selection without config
UNBLOCK: Complete Phase 2.1 first
```

### Scenario 2: Commands Not Copied
```
BLOCKS: All of Phase 3
REASON: Cannot modify files that don't exist
UNBLOCK: Complete Phase 1.1 first
```

### Scenario 3: Workflow Not Independent
```
BLOCKS: Phase 3 (all commands), Phase 6 (testing)
REASON: Commands reference workflow, agent selection broken
UNBLOCK: Complete Phase 1.2 first
```

### Scenario 4: Installer Not Working
```
BLOCKS: Phase 6.2 (integration testing)
REASON: Cannot test installation without installer
UNBLOCK: Complete Phase 2.2 first
```

## Parallel Execution Strategy

### Strategy 1: Two-Person Team
```
Person A:                    Person B:
─────────────────────────    ─────────────────────────
Phase 1.1 (Commands)         Phase 1.2 (Workflow)
         ↓                          ↓
Phase 3.1 (setup.md)         Phase 3.2 (newTrack.md)
         ↓                          ↓
Phase 3.3 (implement.md)    Phase 3.4 (remaining)
         ↓                          ↓
Phase 4.1 (Scripts)          Phase 5.1 (SKILL.md)
         ↓                          ↓
Phase 6.1 (Unit tests)       Phase 6.2 (Integration)
         ↓                          ↓
         └─────── Phase 7 ─────┘
```

### Strategy 2: Single Developer (Maximize Parallelism)
```
Day 1: Phase 1.1, 1.2 (sequential)
Day 2: Phase 1.3 || Phase 2.1 → Phase 2.2
Day 3: Phase 3.1 → Phase 3.2 || 3.3 || 3.4
Day 4: Phase 4 → Phase 5.1 || 5.2 || 5.3
Day 5: Phase 6.1 || 6.2 || 6.3 → Phase 7
```

## Timeline Visualization

```
Week 1:
  Mon:   Phase 1.1 (2-3h)
  Tue:   Phase 1.2 (1-2h) + Phase 1.3 || Phase 2.1 (2-3h)
  Wed:   Phase 2.2 (2-3h)
  Thu:   Phase 3.1 (2h)
  Fri:   Phase 3.2 || 3.3 || 3.4 (4-5h)

Week 2:
  Mon:   Phase 4 (1-2h)
  Tue:   Phase 5.1 || 5.2 || 5.3 (2-3h)
  Wed:   Phase 6.1 || 6.2 || 6.3 (3-4h)
  Thu:   Phase 7 (1h) + Buffer
  Fri:   Release 🎉
```

## Critical Milestones

### Milestone 1: Foundation Complete
**When**: End of Phase 1
**Deliverables**:
- 8 command files created
- Independent workflow.md
- All code styleguides copied
**Validation**: `test -f` checks pass for all files

### Milestone 2: Integration Ready
**When**: End of Phase 2
**Deliverables**:
- opencode.jsonc.example valid
- install-opencode.sh functional
**Validation**: `jq empty` passes, installer runs in dry-run

### Milestone 3: Commands Adapted
**When**: End of Phase 3
**Deliverables**:
- All commands reference OpenCode agents
- No Claude Code remnants
**Validation**: `grep -r "conductor|/maestro:"` returns empty

### Milestone 4: Testing Complete
**When**: End of Phase 6
**Deliverables**:
- All tests passing
- Coverage >90%
**Validation**: Test suite passes, coverage report complete

### Milestone 5: Release Ready
**When**: End of Phase 7
**Deliverables**:
- Git tag created
- Release notes published
**Validation**: Tag exists in repository

## Dependency Summary

### Hard Dependencies (Must Follow Order)
1. Phase 1.1 → Phase 3
2. Phase 1.2 → Phase 3
3. Phase 2.1 → Phase 3
4. Phase 3 → Phase 4
5. Phase 4 → Phase 5
6. Phase 5 → Phase 6
7. Phase 6 → Phase 7

### Soft Dependencies (Can Parallelize)
1. Phase 1.3 ║ Phase 2.1
2. Phase 3.2, 3.3, 3.4 (after 3.1)
3. Phase 5.1, 5.2, 5.3 (after 4)
4. Phase 6.1, 6.2, 6.3 (after 5)

### No Dependencies (Can Start Anytime)
- Documentation review (can read while implementing)
- Test script preparation (can write while testing)
- Release notes draft (can prepare early)

---

**Graph Status**: COMPLETE
**Next Action**: Begin Phase 1.1 after answering pre-implementation questions
**Contact**: Reference IMPLEMENTATION_PLAN.md for detailed task descriptions
