# Swift Guide

When I write Swift, I default to value semantics, explicit state transitions, and APIs that feel Swifty without becoming obscure.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Clear modeling of state, identity, and ownership.
- Safety through types, optionals, and compiler guidance.
- Structured concurrency and predictable async behavior.
- UI code that stays declarative and testable where the platform allows it.

## Required defaults

- Prefer structs and enums by default; use classes when shared identity or reference semantics are truly part of the model.
- Use `guard` for early exits and invariant enforcement when it makes the happy path easier to read.
- Keep optionals honest and avoid force-unwrapping outside tiny, obvious invariants or test code.
- Use async/await and task structure deliberately, with cancellation considered for longer workflows.
- Keep extensions focused and avoid scattering one type's core behavior across many unrelated files.

## Architecture

- Separate domain models, side-effecting services, and UI-facing adapters or view models.
- Use protocol abstractions where multiple implementations or test seams are real, not by default everywhere.
- Represent UI state explicitly instead of juggling many booleans that can drift out of sync.
- In SwiftUI code, keep views mostly declarative and move workflow logic out of the body builder.

## Verification

- Test pure logic and state transitions directly, then add targeted integration or UI coverage where it buys confidence.
- Use previews or fixtures to exercise empty, loading, error, and content-heavy states.
- Watch main-thread work, task lifetime, and retained references in async-heavy code.
- Name methods and tests for behavior, not for the UI control or framework hook they happen to touch.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Force unwraps sprinkled through production code.
- Reference types used by habit when values would be simpler and safer.
- Protocol proliferation with no concrete need.
- Massive view controllers or view models that become the new global god object.
