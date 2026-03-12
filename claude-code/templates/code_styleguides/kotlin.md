# Kotlin Guide

When I write Kotlin, I lean on null safety, expression-oriented code, and data modeling that makes impossible states hard to represent.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Concise code that stays explicit about behavior and ownership.
- Domain models that use sealed types, data classes, and nullability intentionally.
- Structured concurrency instead of ad hoc async work.
- Thin framework layers around testable core logic.

## Required defaults

- Prefer immutable `val` state and data classes for transport or domain records.
- Use nullable types honestly and avoid `!!` outside narrow, obvious invariants.
- Reach for sealed interfaces or classes when a workflow has a finite set of states or outcomes.
- Use extension functions sparingly for true readability wins, not to hide unrelated behavior behind cute syntax.
- Prefer constructor injection and explicit dependencies over service locator patterns.

## Architecture

- Keep coroutines scoped to a lifecycle with clear cancellation and supervision rules.
- Separate mapping, orchestration, and business rules so each change has one obvious home.
- Use small interfaces and composition; do not rebuild enterprise inheritance trees in Kotlin syntax.
- In Android code, keep UI state explicit and one-way where practical.

## Verification

- Test domain logic without framework boot when possible.
- Use integration tests for database, HTTP, serialization, and coroutine boundary behavior.
- Name tests for the behavior and state transition they protect.
- Watch coroutine leaks and dispatcher misuse in long-lived services or UI flows.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Nested scope functions that turn control flow into punctuation.
- Excessive operator overloading or DSL cleverness in ordinary application code.
- `!!` as a casual escape hatch.
- State mutation from many places without a clear owner.
