# Shell Guide

When I write shell scripts, I assume they will eventually run in the wrong directory, with the wrong input, at the worst possible time. I write them accordingly.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Predictable automation with obvious failure behavior.
- Small scripts that orchestrate tools rather than reimplementing them badly.
- Safe handling of paths, quoting, and exit codes.
- Portability when the environment requires it, explicit Bash usage when it does not.

## Required defaults

- Use `#!/usr/bin/env bash` for Bash scripts and make the shell requirement explicit.
- Start Bash scripts with `set -euo pipefail` unless there is a specific reason not to, and then document that reason.
- Quote variable expansions unless I explicitly need word splitting or globbing.
- Use functions for meaningful steps and keep each function focused.
- Prefer `printf` over `echo` when output must be exact.

## Architecture

- Use shell as glue. When logic becomes data-heavy or stateful, move it to a more suitable language.
- Resolve paths from the script location when the script depends on sibling files.
- Check prerequisites early and fail with clear messages.
- Keep temporary files and cleanup behavior explicit, ideally with traps.

## Verification

- Run `shellcheck` and treat its warnings seriously.
- Test scripts with paths containing spaces, empty inputs, and missing dependencies.
- Make destructive actions opt-in, dry-runnable, or at least loudly visible.
- Surface stderr from called tools unless silence is intentional and safe.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Parsing `ls`, unquoted expansions, and relying on implicit current-directory state.
- Silent fallthrough after failed commands in multi-step workflows.
- Huge one-liners that are impossible to debug later.
- Using shell for complex parsing, concurrency, or long-lived application logic.
