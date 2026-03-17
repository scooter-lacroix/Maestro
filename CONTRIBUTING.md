# Contributing to Maestro

## Before You Start

Maestro is a multi-surface project:

- `maestro` is the orchestration layer
- `leindex` is the standalone code exploration and analysis tool
- `tracklens` is the walkthrough/review surface
- `cockpit` is the TUI control plane

If your change only affects one surface, keep the change scoped to that surface.

## Good Contributions

The most useful contributions are:

- bug fixes with a clear reproduction
- docs that reduce setup or usage confusion
- focused UX improvements to onboarding and repo discoverability
- tests that close real gaps
- security or reliability hardening

Please avoid bundling unrelated refactors into feature work.

## Development Workflow

1. Create a focused branch.
2. Make the smallest coherent change that solves the problem.
3. Add or update tests when behavior changes.
4. Update docs when user-facing behavior changes.
5. Run the relevant validation before opening a PR.

## Validation

Use the narrowest validation that honestly covers your change. For larger cross-cutting work, run:

```bash
cargo test --workspace
bun run build:tracklens
```

If you touch only one crate or one surface, run the targeted checks for that area and say what you ran.

## Pull Requests

PRs should make review easy:

- explain the problem
- explain the change
- list validation performed
- call out risks or follow-up work

Keep PRs reviewable. Smaller PRs merge faster and break less.

## Documentation Expectations

If you change install flow, onboarding, CLI behavior, or the project’s public positioning, update:

- `README.md`
- `docs/INSTALLATION.md` when installation behavior changes
- any affected surface-specific docs

## Security

Do not open public issues for vulnerabilities. Follow [SECURITY.md](SECURITY.md).

## Code Style

- Prefer minimal, local changes over broad rewrites.
- Preserve existing behavior unless the change explicitly intends to alter it.
- Do not leave dead code behind unless it is immediately used.
- Keep user-facing docs clear and concrete.
