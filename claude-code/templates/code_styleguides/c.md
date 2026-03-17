# C Guide

When I write C, I optimize for correctness, explicit ownership, and code that another engineer can audit with confidence.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Simple control flow and obvious data ownership.
- Clear boundaries between allocation, mutation, and cleanup.
- Small, testable units with few hidden dependencies.
- Defensive handling of errors, lengths, and lifetimes.

## Required defaults

- Use `clang-format` or the project formatter and keep style machine-enforced.
- Prefer `static` for internal linkage and small file-scoped APIs.
- Pass lengths with buffers and keep ownership rules documented in the function contract.
- Use `const` aggressively to communicate read-only intent.
- Initialize variables close to use and zero-initialize structs when it clarifies starting state.

## Architecture

- Keep headers minimal: declare interfaces, hide internals, and avoid dragging heavy dependencies through every compile unit.
- Use one cleanup path for resources, usually with a `goto cleanup` pattern when a function owns several resources.
- Wrap unsafe or repetitive patterns behind tight helper functions instead of copy-pasting pointer arithmetic everywhere.
- Prefer explicit data structures over macro-heavy pseudo-frameworks.

## Verification

- Compile with warnings-as-errors and keep sanitizers available in development builds.
- Test edge cases around buffer sizes, null inputs, truncation, and partial failure.
- Measure performance before introducing lower-level tricks; many 'optimizations' just make bugs harder to find.
- Use assertions for programmer errors and normal error returns for runtime failures.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Unchecked casts, unchecked allocation results, and ambiguous ownership.
- Macros that behave like functions but hide side effects or evaluate arguments multiple times.
- Global mutable state unless the problem is truly process-wide and synchronization is explicit.
- Deeply nested functions that mix parsing, business rules, IO, and cleanup.
