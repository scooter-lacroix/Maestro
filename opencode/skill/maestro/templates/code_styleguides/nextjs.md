# Next.js Guide

When I write Next.js, I use the framework's server-first strengths deliberately and keep client code small, explicit, and worth its cost.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Server-rendered data flow by default.
- Clear boundaries between server-only code, client interactivity, and shared pure utilities.
- Fast initial render and predictable cache behavior.
- Feature-local structure that keeps routes understandable.

## Required defaults

- Use the App Router and server components by default.
- Add `'use client'` only when a component truly needs browser-only hooks, local interactivity, or imperative APIs.
- Fetch data on the server whenever possible and keep client components focused on presentation and interaction.
- Treat route handlers, server actions, and page components as orchestration layers; push reusable logic into shared modules.
- Make caching explicit so revalidation behavior is intentional rather than surprising.

## Architecture

- Organize routes by feature and shared UI by real reuse, not by imagined reuse.
- Keep forms, mutations, and validation close together so the full workflow is visible.
- Use typed schemas for request parsing and environment validation.
- Respect server-only boundaries for secrets, heavy dependencies, and trusted operations.

## Verification

- Test pure logic outside the framework first, then add integration coverage for routing, auth, and mutations.
- Watch bundle size and hydration cost; every client component should justify itself.
- Verify loading, empty, not-found, and error states for route segments.
- Use logging and metrics on server actions or route handlers that touch important workflows.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Turning the entire app into client components out of convenience.
- Leaking secrets or privileged logic into shared modules imported by the client.
- Global state by default when route-local or feature-local state is enough.
- Caching behavior left to guesswork.
