# Java Guide

When I write Java, I favor explicit design, strong domain types, and modern language features that reduce ceremony rather than increase it.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Code that is easy to reason about in large teams and long-lived systems.
- Clear package boundaries and explicit dependencies.
- Business rules that are not buried in framework annotations or infrastructure layers.
- Pragmatic use of modern Java features like records, switch expressions, and sealed types.

## Required defaults

- Prefer constructor injection and immutable fields for required collaborators.
- Use records for simple immutable carriers and sealed hierarchies when a state machine or variant set is closed.
- Keep methods short and single-purpose; split orchestration from domain logic.
- Model absence intentionally with `Optional` at boundaries where it adds clarity, not everywhere by default.
- Use checked exceptions sparingly and only when the caller is expected to recover in a meaningful way.

## Architecture

- Organize by feature or bounded context rather than giant top-level folders of controllers, services, and repositories.
- Keep framework-facing code thin so the core behavior can be tested without a container.
- Prefer explicit mappings between transport models and domain models.
- Avoid deep inheritance; composition and small interfaces usually age better.

## Verification

- Test domain code without Spring or other heavy runtimes when possible.
- Use focused integration tests for persistence, messaging, HTTP, and serialization boundaries.
- Validate configuration at startup and fail early on missing required settings.
- Instrument services with structured logs, metrics, and timeouts rather than hoping stack traces will be enough later.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Anemic domain models plus giant service classes that do everything.
- Over-annotated code where behavior is hard to trace from reading the class.
- Static singletons and hidden global state.
- Ceremony-heavy patterns copied from old enterprise templates when a simpler design works.
