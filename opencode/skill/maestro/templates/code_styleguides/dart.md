# Dart Guide

When I write Dart, I rely on sound null safety, immutable models, and small composable units. If the project is Flutter, those same rules carry into the widget tree.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Predictable state transitions and readable async flows.
- Types that make nullability and domain constraints explicit.
- Feature-local organization instead of giant utility bins.
- Code that is equally maintainable in server and Flutter contexts.

## Required defaults

- Keep null safety fully enabled and avoid `!` unless the invariant is local and obvious.
- Prefer immutable classes and `final` fields; mutation should be deliberate and easy to track.
- Use named parameters for clarity when functions take more than a couple of related values.
- Favor `async`/`await` for request-style flows and streams only when the problem is genuinely stream-shaped.
- In Flutter, use `const` constructors, small widgets, and derived view state instead of manual syncing.

## Architecture

- Split domain logic, infrastructure, and presentation so widgets or handlers stay thin.
- Keep models explicit; map JSON at boundaries and avoid leaking loose maps through the app.
- Use extension methods sparingly for true language-level readability wins, not as a dumping ground.
- Prefer one clear state-management pattern per app or feature rather than mixing several paradigms.

## Verification

- Test pure domain logic heavily and UI state transitions at a focused level.
- Use widget or integration tests for behavior that spans navigation, rendering, or async orchestration.
- Watch rebuild cost in Flutter, but fix measured hotspots instead of cargo-culting micro-optimizations.
- Validate configuration and platform assumptions early during startup.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Passing around `Map<String, dynamic>` as if it were a domain model.
- Large stateful widgets or monolithic service classes.
- Manual null assertions sprinkled through the codebase.
- Framework churn for its own sake when the current structure is already clear and testable.
