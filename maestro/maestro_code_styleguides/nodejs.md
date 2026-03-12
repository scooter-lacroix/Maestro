# Node.js Guide

When I write Node.js services, I keep the event loop healthy, the startup path explicit, and infrastructure concerns from bleeding into domain code.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Predictable async behavior and explicit process lifecycle management.
- Fast startup, graceful shutdown, and structured observability.
- Thin transport layers with testable business logic underneath.
- Runtime safety at boundaries through validation and configuration discipline.

## Required defaults

- Prefer TypeScript for non-trivial services, but the same design rules apply in JavaScript.
- Validate environment variables and startup dependencies before serving traffic.
- Use `async`/`await`, explicit timeouts, and cancellation or abort support where libraries allow it.
- Keep the app creation path separate from the process boot path so tests can instantiate the app without opening sockets.
- Use structured logs with request or job correlation identifiers.

## Architecture

- Group by feature or domain rather than giant folders of controllers, services, and repositories unless the codebase genuinely benefits.
- Keep request handlers thin; parse, authorize, call domain logic, format response.
- Wrap external APIs, queues, and data stores behind clear interfaces owned by the consuming code.
- Treat concurrency, retries, and background jobs as first-class workflow design, not afterthoughts.

## Verification

- Test domain logic without HTTP or framework boot when possible.
- Use integration tests for database, cache, message bus, and HTTP boundaries.
- Watch for event-loop blocking, unbounded concurrency, and memory growth on large collections or streams.
- Handle signals and shutdown deliberately so the service can drain work cleanly.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Mixing startup side effects into module top level code.
- Unhandled promise rejections, fire-and-forget async work, and retries with no policy.
- Passing raw request objects deep into domain code.
- A dependency graph so tangled that one small feature bootstraps the world.
