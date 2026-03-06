# TrackLens Full Port Source Review (Tzar Directive)

- Date: 2026-03-04
- Track: `maestro/tracks/tracklens-fullport_20260304`
- Reviewer: Codex
- Method: Source-code investigation only (no track notes / git notes used as truth source)
- Required navigation tool usage: LeIndex CLI used throughout

## LeIndex CLI Usage (mandatory)

Executed (representative subset):
- `leindex diagnostics -p /mnt/WD-SSD/Prod/maestro`
- `leindex index /mnt/WD-SSD/Prod/maestro --progress`
- `leindex search "tracklens_review" -p /mnt/WD-SSD/Prod/maestro --top-k 3`
- `leindex search "startTrackLensServer" -p /mnt/WD-SSD/Prod/maestro --top-k 3`
- `leindex search "TRACKLENS_AUTH_TOKEN" -p /mnt/WD-SSD/Prod/maestro --top-k 3`
- `leindex analyze "TrackLens Node server implementation..." -p /mnt/WD-SSD/Prod/maestro --tokens 5000`

Note: LeIndex returned some noisy semantic matches from unrelated areas; final judgments were confirmed by direct file-level source inspection and reproducible build/test commands.

## Task-by-Task Source Assessment

Legend:
- `Verified`: implemented and source-backed
- `Partial`: implemented but materially incomplete
- `Missing`: not implemented
- `Incorrect claim`: task marked done in `plan.md` but source/tests contradict
- `Pending`: intentionally unchecked manual verification task

### Phase 1 - Foundation & Rebranding

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Create TrackLens package/layout scaffolds | [x] | Verified | Directories exist: `apps/tracklens-hook`, `apps/tracklens-opencode`, `packages/tracklens-server`, `packages/tracklens-editor`, `packages/tracklens-review-editor`, `packages/tracklens-ui`, `packages/tracklens-shared`, `packages/tracklens-web-highlighter`, `pi-maestro/src/tracklens`, `src/leindex/src/tracklens`, `crates/cockpit/src/tracklens`, `crates/cli/src/commands` |
| Port core UI types/utils (`types.ts`, parser, storage, identity, annotationHelpers, planDiffEngine, editorMode) | [x] | Verified | Files exist under `packages/tracklens-ui/src/types.ts`, `packages/tracklens-ui/src/utils/parser.ts`, `packages/tracklens-ui/src/utils/storage.ts`, `packages/tracklens-ui/src/utils/identity.ts`, `packages/tracklens-ui/src/utils/annotationHelpers.ts`, `packages/tracklens-ui/src/utils/planDiffEngine.ts`, `packages/tracklens-ui/src/utils/editorMode.ts` |
| Rebrand constants/paths/env (`plannotator` -> `tracklens`, env renames, storage path, package scope) | [x] | Partial | Renames present in `packages/tracklens-server/src/remote.ts:5`, `packages/tracklens-server/src/browser.ts:5`, `packages/tracklens-server/src/storage.ts:32`; package scopes are `@maestro/tracklens-*`; legacy strings still present in source/comments/migration references (grep not zero) |
| Remove sharing/paste/marketing/mascot assets; migrate legacy localStorage keys | [x] | Partial | Sharing UI remnants still exist: `packages/tracklens-ui/src/components/ImportModal.tsx:79`, `packages/tracklens-ui/src/components/ImportModal.tsx:94`, `packages/tracklens-ui/src/hooks/useSharing.ts:13`; legacy migration exists only in some areas (example: `packages/tracklens-ui/src/utils/autonomyMode.ts:21`) |
| Rebranding audit script (grep for forbidden strings) | [x] | Incorrect claim | No dedicated audit script found; forbidden strings remain in source/comments/tests (e.g. `packages/tracklens-ui/src/utils/autonomyMode.ts:21`, `apps/tracklens-opencode/src/index.test.ts:33`) |
| Maestro user manual verification: Foundation & Rebranding | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:10` |

### Phase 2 - Server Layer (Node)

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Port Node server (`startTrackLensServer/review/annotate`, env remap, remove share/paste routes) | [x] | Partial | Implementations exist in `packages/tracklens-server/src/index.ts:107`, `packages/tracklens-server/src/review.ts:54`, `packages/tracklens-server/src/annotate.ts:45`; route surface still has security and contract issues (see critical findings) |
| Update integrations/frontmatter helpers to TrackLens tags | [x] | Partial | Rebranding done in `packages/tracklens-server/src/integrations.ts:33`, `packages/tracklens-server/src/integrations.ts:81`; unsafe path joining remains (`packages/tracklens-server/src/integrations.ts:225`) |
| Bun unit tests for server layer | [x] | Incorrect claim | No server tests exist; `bun test` in `packages/tracklens-server` returns "No tests found" |
| Maestro user manual verification: Server Layer | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:16` |

### Phase 3 - UI Components

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Port editor/review-editor/web-highlighter apps | [~] | Partial | Apps exist; review-editor uses demo data only (`packages/tracklens-review-editor/src/App.tsx:31`) and submit is local-only (`packages/tracklens-review-editor/src/App.tsx:74`); all package READMEs still "Under Construction" (`packages/tracklens-editor/README.md:29`, `packages/tracklens-review-editor/README.md:29`, `packages/tracklens-web-highlighter/README.md:25`) |
| Port settings/autonomy (`AutonomyModeSetup`, settings cleanup, completion labels) | [ ] | Missing | No `AutonomyModeSetup` component; permission mode remains primary (`packages/tracklens-editor/src/App.tsx:24`, `packages/tracklens-ui/src/components/Settings.tsx:44`, `packages/tracklens-ui/src/components/PermissionModeSetup.tsx:1`) |
| Update utils (`autonomyMode`, `docSave`, `uiPreferences`, `obsidian`, `bear`, `defaultNotesApp`, `agentSwitch`, `useVaultBrowser`, `useAgents`) | [ ] | Partial | Some utils exist, but migration is inconsistent: dual save systems (`packages/tracklens-ui/src/utils/planSave.ts:1`, `packages/tracklens-ui/src/utils/docSave.ts:1`), exports still prioritize planSave (`packages/tracklens-ui/src/index.ts:42`), vault browser fixed to `tracklens` folder (`packages/tracklens-ui/src/hooks/useVaultBrowser.ts:44`) |
| Build single-file Vite bundle for hook HTML in `apps/tracklens-hook` | [ ] | Missing | Hook server expects `../dist/index.html`, `review.html`, `annotate.html` (`apps/tracklens-hook/server/index.ts:38`, `apps/tracklens-hook/server/index.ts:42`, `apps/tracklens-hook/server/index.ts:46`), but `apps/tracklens-hook/dist` has only JS/DTS artifacts |
| Bun UI sanity tests | [ ] | Missing | No app-level UI sanity tests in editor/review/web-highlighter; `packages/tracklens-ui` tests are export-only (`packages/tracklens-ui/test/index.test.ts:10`) and package build currently fails (`packages/tracklens-ui/src/components/ExportModal.tsx:87`) |
| Maestro user manual verification: UI Components | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:24` |

### Phase 4 - Claude Code Integration

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Hook binding + manifests + CLI modes using TrackLens server functions | [x] | Partial | Required files exist (`apps/tracklens-hook/hooks/hooks.json`, `apps/tracklens-hook/.claude-plugin/plugin.json`, `apps/tracklens-hook/server/index.ts`), but runtime artifacts expected by server are missing (no hook HTML files in dist) |
| Slash commands `/tracklens-review.md` and `/tracklens-annotate.md` | [x] | Verified | Present at `apps/tracklens-hook/commands/tracklens-review.md` and `apps/tracklens-hook/commands/tracklens-annotate.md` |
| Smoke tests (stdin hook event, review, annotate) | [ ] | Missing | No hook test files; no smoke harness in `apps/tracklens-hook` |
| Maestro user manual verification: Claude Code Integration | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:30` |

### Phase 5 - OpenCode Integration

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Port OpenCode plugin with tools (`tracklens`, `tracklens-review`, `tracklens-annotate`) and settings | [x] | Incorrect claim | Source registers only `submit_plan` schema (`apps/tracklens-opencode/src/index.ts:184`), listens only for `tracklens-review` command (`apps/tracklens-opencode/src/index.ts:115`), no `tracklens-annotate` flow; README claims 3 tools (`apps/tracklens-opencode/README.md:8`) but implementation does not match |
| Bun tests for OpenCode plugin | [x] | Incorrect claim | `bun test` fails 2 tests in `apps/tracklens-opencode/src/index.test.ts:41` (expects `startTrackLensServer`, source uses `startReviewServer`) |
| Maestro user manual verification: OpenCode Integration | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:35` |

### Phase 6 - Pi-mono + newTrack/implement Wiring

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Register `tracklens_review` and `tracklens_walkthrough` tools | [x] | Partial | Tools registered in `pi-maestro/src/tracklens/extension/tools.ts:36` and `pi-maestro/src/tracklens/extension/tools.ts:241`; `tracklens_review` fallback path likely wrong (`pi-maestro/src/tracklens/extension/tools.ts:125`), `tracklens_walkthrough` returns markdown only (no UI decision loop) |
| Modify `newTrack.ts` with checkpoints 3.6, 4.5, 5.7 | [x] | Partial | Checkpoint text exists (`pi-maestro/src/commands/newTrack.ts:243`), but references `maestro/tracks/<track_id>/spec.md` before track artifacts are created (`pi-maestro/src/commands/newTrack.ts:247` vs creation at `pi-maestro/src/commands/newTrack.ts:306`) |
| Modify `implement.ts` walkthrough loop + default-on toggle + fallback | [x] | Partial | Prompt text says `/tracklens toggle on/off` (`pi-maestro/src/commands/implement.ts:325`) but command parser only supports `/tracklens on|off` (`pi-maestro/src/tracklens/extension/command.ts:36`); toggle state is not consumed outside command module |
| Pi-mono tests for tools/workflows | [ ] | Missing | No tests for tool/command wiring; only walkthrough generator/storage tests exist under `pi-maestro/src/tracklens/walkthrough/test` |
| Maestro user manual verification: Pi-mono + Workflow Wiring | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:42` |

### Phase 7 - Walkthrough System

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Implement TS walkthrough generator | [x] | Partial | Generator exists (`pi-maestro/src/tracklens/walkthrough/generator.ts`), but git-range logic is weak (`findTrackStartCommit` grep + `${sinceCommit}^..HEAD` at `pi-maestro/src/tracklens/walkthrough/generator.ts:353`, `pi-maestro/src/tracklens/walkthrough/generator.ts:436`) |
| Implement storage/compression (`walkthrough.compressed` + `walkthrough-final.md`) | [x] | Partial | Storage exists (`pi-maestro/src/tracklens/walkthrough/storage.ts:32`), but artifact is JSON with `compressed` field (not dedicated `walkthrough.compressed` file); compression decode robustness issue in shared codec (`packages/tracklens-shared/src/compress.ts:33`) |
| Tests for generator/storage/remediation flow | [x] | Incorrect claim | Tests cover generator/storage only (`pi-maestro/src/tracklens/walkthrough/test/generator.test.ts`, `pi-maestro/src/tracklens/walkthrough/test/storage.test.ts`); no remediation-loop tests |
| Integrate denial remediation loop | [x] | Partial | `runRemediationLoop` exists but is not wired to `tracklens_walkthrough` tool; `executeRemediationTasks` is placeholder logging (`pi-maestro/src/tracklens/walkthrough/remediation.ts:275`) |
| Maestro user manual verification: Walkthrough System | [x] | Incorrect claim | Claimed completed, but test logs show git failures during tests (`fatal: not a git repository` in walkthrough tests) and no manual verification artifact |

### Phase 8 - Rust/Cockpit/CLI

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Rust TrackLens module (`types.rs`, `server.rs`, `walkthrough.rs`, export via lib.rs) | [x] | Partial | Module exists (`src/leindex/src/tracklens` + `src/leindex/src/lib.rs:36`), but API contract mismatches JS editor (`/api/content` vs `/api/plan` and decision schema mismatch), plus predictable auth token generation (`src/leindex/src/tracklens/server.rs:111`) |
| Cockpit pane and tab wiring | [x] | Partial | Pane and render tab exist (`crates/cockpit/src/tracklens/mod.rs`, `crates/cockpit/src/tabs/tracklens.rs`), but no runtime callsites to `start_review/complete_review`; tab cycling still modulo 10 with 11 tabs (`crates/cockpit/src/app.rs:3199`, `crates/cockpit/src/app.rs:4269`) |
| CLI subcommand and registration; bundle distribution flow | [x] | Partial | Command registered (`crates/cli/src/main.rs:172`, `crates/cli/src/commands/tracklens.rs`), but save-path guard is brittle/non-functional on first write (`crates/cli/src/commands/tracklens.rs:253`) and no automated copy/distribution flow for HTML bundle found in source |
| Cargo tests for tracklens and cockpit integration | [x] | Partial | `cargo test -p maestro-cockpit tracklens` passes pane tests; `cargo test -p maestro-cli tracklens` passes command tests; `cargo test -p leindex-core tracklens` fails linker (`rust-lld ... invalid symbol index`) |
| Maestro user manual verification: Rust/Cockpit/CLI | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:56` |

### Phase 9 - Rollout, QA, Docs

| Plan task | Plan state | Source verdict | Evidence |
|---|---|---|---|
| Tzar review remediation: fixed all 10 critical security issues | [x] | Incorrect claim | Critical security and contract defects still present: unauthenticated Node decisions, path traversal surfaces, Rust/JS API mismatch, predictable auth token, cockpit state wiring gaps |
| E2E manual checkpoints (newTrack approvals, walkthrough deny/remediate/approve, toggle behavior) | [ ] | Missing | No E2E/manual artifacts or test coverage found in source |
| Final rebranding audit + document fallback + zero forbidden strings | [ ] | Missing | Forbidden strings still present in scoped source/comments/tests; no dedicated audit script |
| Update docs (commands/env/toggles/workflow/acceptance checklist) | [ ] | Missing | TrackLens command/toggle docs not updated in project docs; track package READMEs still under-construction |
| Maestro user manual verification: Rollout, QA, Docs | [ ] | Pending | Unchecked in `maestro/tracks/tracklens-fullport_20260304/plan.md:63` |

## Required Output (Tzar)

### 1) Critical Issues List (must fix before proceeding)

1. Rust server/JS editor API contract is broken: server exposes `GET /api/content` and decision schema `{behavior: allow|deny}`, while editor expects `GET /api/plan` and posts `{approved: boolean,...}` without auth. Evidence: `src/leindex/src/tracklens/server.rs:132`, `src/leindex/src/tracklens/server.rs:133`, `packages/tracklens-editor/src/App.tsx:61`, `packages/tracklens-editor/src/App.tsx:131`.
2. Rust decision endpoint requires bearer auth token, but editor never sends one. Evidence: `src/leindex/src/tracklens/server.rs:253`, `packages/tracklens-editor/src/App.tsx:133`.
3. Rust auth token generation is predictable timestamp hex, not cryptographically secure. Evidence: `src/leindex/src/tracklens/server.rs:111`.
4. Node decision endpoints are unauthenticated in plan/review/annotate servers. Evidence: `packages/tracklens-server/src/index.ts:410`, `packages/tracklens-server/src/review.ts:236`, `packages/tracklens-server/src/annotate.ts:185`.
5. Node vault/integration endpoints permit path traversal via unsanitized `folder` joins. Evidence: `packages/tracklens-server/src/index.ts:347`, `packages/tracklens-server/src/integrations.ts:225`.
6. Claude hook runtime references missing HTML artifacts (`index.html`, `review.html`, `annotate.html`) in `apps/tracklens-hook/dist`, so review modes are not runnable as implemented. Evidence: `apps/tracklens-hook/server/index.ts:38`, `apps/tracklens-hook/server/index.ts:42`, `apps/tracklens-hook/server/index.ts:46`; dist contents contain only JS/DTS.
7. OpenCode integration does not implement spec-required tool coverage; only `submit_plan` schema is registered and no execute path is provided for plan-review workflow. Evidence: `apps/tracklens-opencode/src/index.ts:184`, `apps/tracklens-opencode/src/index.ts:115`.
8. Walkthrough remediation is not operational end-to-end: tool path does not invoke remediation loop, and executor is placeholder logging only. Evidence: `pi-maestro/src/tracklens/extension/tools.ts:241`, `pi-maestro/src/tracklens/walkthrough/remediation.ts:275`.
9. Cockpit TrackLens state is not wired to real runtime events (`start_review/complete_review` never called outside tests). Evidence: `crates/cockpit/src/tracklens/mod.rs:69`, `crates/cockpit/src/tracklens/mod.rs:81`, reference search found no non-test callsites.
10. Cockpit tab navigation still uses modulo 10 while tab count is 11, so TrackLens tab navigation is inconsistent/unreachable via next-tab cycle. Evidence: `crates/cockpit/src/app.rs:69`, `crates/cockpit/src/app.rs:3199`, `crates/cockpit/src/app.rs:4269`.
11. CLI walkthrough save-path security check can block valid first write because it canonicalizes non-existent output and compares relative-to-absolute path. Evidence: `crates/cli/src/commands/tracklens.rs:253`.
12. Claimed server-layer and OpenCode automated test status is incorrect: server has no tests; OpenCode tests currently fail. Evidence: `packages/tracklens-server` test run output, `apps/tracklens-opencode/src/index.test.ts:41`.

### 2) Improvements Needed (should fix for excellence)

1. Replace permission-mode-first UX with autonomy-first UX and component naming consistency (`AutonomyModeSetup`, settings labels, payload field naming).
2. Remove or fully retire sharing/import artifacts (`useSharing`, share-link copy in ImportModal) to match no-sharing NFR.
3. Resolve duplicate save abstractions (`planSave` vs `docSave`) and export only one canonical API.
4. Fix hook build pipeline (`apps/tracklens-hook/package.json`) and generate required HTML artifacts in the expected dist location.
5. Align OpenCode README claims with actual code or implement missing tools/handlers.
6. Add real tests for Node server routes, auth checks, path validation, and OpenCode command/tool execution behavior.
7. Add Pi-mono tests for `tracklens_review`, `tracklens_walkthrough`, `/tracklens` command parsing, and newTrack/implement checkpoint behavior.
8. Correct newTrack checkpoint sequencing so file paths referenced in review checkpoints exist before invocation.
9. Ensure track toggle state is actually consumed by implement flow.
10. Replace under-construction READMEs with current operational status and limitations.

### 3) Optimization Opportunities

1. Walkthrough generator runs multiple `git` subprocesses per file (`numstat`, `diff`, snippet extraction) and can be batched to reduce process overhead. Evidence: `pi-maestro/src/tracklens/walkthrough/generator.ts:381`, `pi-maestro/src/tracklens/walkthrough/generator.ts:397`.
2. Rust server `wait_for_decision` is a polling loop every 100ms; use notify/channel primitive to avoid wake-loop overhead. Evidence: `src/leindex/src/tracklens/server.rs:185`.
3. Compression code builds a JS binary string char-by-char; move to typed-array/base64 path with lower allocation churn for large walkthroughs. Evidence: `packages/tracklens-shared/src/compress.ts:22`.
4. Minimize duplicate compiled tests in OpenCode package (`dist/index.test.js` being executed alongside source tests) to speed CI and avoid duplicate failures.
5. Limit vault tree scans or add depth/file caps to prevent expensive recursion on large vaults. Evidence: `packages/tracklens-server/src/index.ts:356`.

### 4) Edge Cases Not Handled

1. Walkthrough tests run outside git repos and still pass despite fatal git errors, masking production regressions. Evidence: walkthrough test output and `pi-maestro/src/tracklens/walkthrough/generator.ts:424`.
2. `findTrackStartCommit` uses grep by track name; similarly named commits can produce wrong base commit, causing unrelated diffs in walkthroughs. Evidence: `pi-maestro/src/tracklens/walkthrough/generator.ts:436`.
3. No remediation behavior coverage when annotations are empty/malformed but denial occurs.
4. Hook/OpenCode HTML asset path expectations are brittle to working-directory/layout differences.
5. UI build strictness catches implicit any in `ExportModal` and currently blocks package build (`packages/tracklens-ui/src/components/ExportModal.tsx:87`).

### 5) Security Concerns

1. Unauthenticated Node decision endpoints (integrity risk).
2. Unsanitized path joins for vault/integration folder inputs (path traversal risk).
3. Predictable Rust auth token generation.
4. JS/Rust contract mismatch around auth means any future token trust model is effectively bypassed by incompatible client.
5. Rebranding/security remediation claims are not traceably backed by source-level controls or tests.

### 6) Performance Issues

1. N+1 git subprocess strategy in walkthrough generation for each file.
2. Poll-based decision waiting in Rust server.
3. Full recursive markdown file collection in vault-tree endpoint with no explicit limits.
4. Large single-file HTML payloads (1.3-1.5MB) with no compression strategy in this layer.

### 7) Final Verdict: FAIL

Reasoning:
- Multiple critical defects remain in API contract, auth/security, runtime artifact delivery, and workflow completeness.
- Several `[x]` claims in `plan.md` are contradicted by current source and test output.
- Core acceptance paths (Rust CLI review flow, hook HTML runtime, OpenCode tool parity, remediation loop) are not production-ready.

Proceeding to approval under Tzar criteria is not defensible until critical issues are remediated and re-validated with source-backed tests and manual checkpoints.

## Validation Commands Executed (source-backed)

- `cd packages/tracklens-server && CI=true bun test` -> No tests found
- `cd apps/tracklens-opencode && CI=true bun test` -> 20 pass, 2 fail (`startTrackLensServer` expectation mismatch)
- `cd apps/tracklens-opencode && CI=true bun run build` -> TS6306 (`tracklens-server` not composite)
- `cd apps/tracklens-hook && CI=true bun run build` -> TS6306 (`tracklens-server` not composite)
- `cd apps/tracklens-hook && CI=true bun run build:ui` -> fails in UI build (`ExportModal` implicit any)
- `cd packages/tracklens-ui && CI=true bun run build` -> TS7034/TS7005 at `ExportModal.tsx`
- `cd packages/tracklens-ui && CI=true bun test` -> 14 pass (export sanity tests only)
- `cd pi-maestro && CI=true bun test src/tracklens/walkthrough/test` -> 21 pass; logs include `fatal: not a git repository`
- `CI=true cargo test -p leindex-core tracklens -- --nocapture` -> linker failure (`rust-lld ... invalid symbol index`)
- `CI=true cargo test -p maestro-cockpit tracklens -- --nocapture` -> tracklens pane tests pass
- `CI=true cargo test -p maestro-cli tracklens -- --nocapture` -> CLI tracklens command tests pass

