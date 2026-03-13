# Go Guide

When I write Go, I favor small packages, straightforward control flow, and APIs that are easy to use correctly.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Code that feels idiomatic to Go engineers, not code transplanted from other languages.
- Simple data flow and explicit error handling.
- Low ceremony around concurrency and package boundaries.
- Tool-enforced consistency through `gofmt`, `go test`, and linters.

## Required defaults

- Let `gofmt` and `goimports` decide formatting and import order.
- Keep package names short, lowercase, and unsurprising.
- Return early on errors and wrap them with context when the caller needs more than the raw failure.
- Define interfaces where they are consumed, not where implementations live.
- Use structs and functions first; add methods when there is real behavior tied to state.

## Architecture

- Keep packages small and cohesive. If two packages always change together, they probably want to be one package.
- Use constructors when a type has invariants or collaborators that must be present.
- Make zero values useful when practical, but do not contort the design to force it.
- Treat goroutines as owned resources: know who starts them, how they stop, and how errors surface.

## Verification

- Table-driven tests are great when they stay readable; I stop using them once the table hides the story.
- Use integration tests for database, network, or concurrency boundaries and keep pure logic easy to test without a harness.
- Profile before tuning allocations or concurrency behavior.
- Plumb `context.Context` through request-scoped work and honor cancellation.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Java-style package layering, giant interfaces, and dependency inversion for its own sake.
- Hiding errors, panicking in library code, or inventing exception-like control flow.
- Concurrency without ownership, shutdown rules, or measured benefit.
- Utility packages that become a junk drawer for unrelated code.
