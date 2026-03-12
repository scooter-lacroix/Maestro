# C# Guide

When I write C#, I lean on the type system, nullable analysis, and clear application boundaries to keep codebase growth under control.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Clear domain models, predictable async behavior, and explicit contracts.
- Small services and handlers that are easy to test in isolation.
- Tooling-enforced consistency instead of hand-maintained style rules.
- Business logic separated from framework glue.

## Required defaults

- Enable nullable reference types and treat warnings as real work, not noise.
- Use `var` when the type is obvious from the right-hand side and the declaration stays readable; otherwise spell the type out.
- Prefer records or immutable DTOs for data that should not drift after creation.
- Use `async`/`await` end to end for IO, returning `Task` or `Task<T>` rather than blocking.
- Favor constructor injection at boundaries and keep container-specific behavior out of domain code.

## Architecture

- Keep controllers, endpoints, and UI handlers thin; orchestration belongs in application services or handlers.
- Represent outcomes explicitly with result types, domain exceptions, or well-shaped error responses instead of null-based guessing.
- Use LINQ where it reads like the business rule; if it turns into puzzle code, switch to named steps.
- Group by feature or bounded context rather than giant folders of classes by technical stereotype.

## Verification

- Test application behavior through public interfaces and focus integration tests on EF, queues, caches, and HTTP edges.
- Log with structure and correlation data so production issues can be traced without log archaeology.
- Keep configuration typed and validated at startup.
- Prefer idempotent background jobs and explicit cancellation support for long-running work.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Static service locators and hidden ambient state.
- Deep inheritance hierarchies when composition or plain functions would do.
- Overusing `dynamic`, reflection, or expression-tree magic in everyday business code.
- Fire-and-forget tasks that silently fail or outlive the request without supervision.
