# PHP Guide

When I write PHP, I aim for explicit application structure, strict types, and domain code that is not trapped inside framework conventions.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Clear boundaries between HTTP, domain, persistence, and templating.
- Strictness and tooling that catch weakly typed mistakes early.
- Readable object design over magical framework coupling.
- Simple data flow through requests, commands, jobs, and views.

## Required defaults

- Enable `declare(strict_types=1);` in application code.
- Use typed properties, constructor injection, and small focused classes.
- Prefer value objects or DTOs over associative arrays once data crosses a meaningful boundary.
- Use PSR-12 formatting and let tooling own style consistency.
- Keep framework facades and globals at the edges, not deep inside domain logic.

## Architecture

- Organize by feature or bounded context when the framework allows it cleanly.
- Keep controllers and route actions thin; they should coordinate, not contain the product rules.
- Make validation, authorization, and persistence explicit rather than relying on implicit side effects.
- Use interfaces only where multiple implementations or testing seams genuinely benefit.

## Verification

- Test domain services and value objects directly, then add integration tests for framework wiring and database behavior.
- Watch query count, serialization cost, and background job idempotency in production-facing paths.
- Validate required configuration at boot and make failure obvious.
- Log with enough structured context to follow a request across workers and jobs.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Passing giant untyped arrays through half the app.
- Fat models or god services that accumulate every use case.
- Framework magic that hides control flow and makes debugging miserable.
- Quietly returning `null` for real failure conditions.
