# React Guide

When I write React, I keep components small, state local by default, and side effects honest about what they synchronize with.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Components that are easy to scan and easy to remove.
- Data flow that stays mostly top-down and explicit.
- A small effect surface area with most logic remaining pure.
- UI code that survives feature growth without turning into hook soup.

## Required defaults

- Use function components and hooks, but not every repeated line needs a custom hook.
- Keep state as close as possible to where it is used.
- Compute derived values during render; do not store them in state just to sync them later.
- Use effects for real synchronization with the outside world, not for ordinary data transformation.
- Prefer explicit props and composition over context or global state until shared ownership is real.

## Architecture

- Split container logic from presentational concerns when it helps readability, not because every component needs a pattern.
- Keep async workflows, mutations, and validation close to the feature that owns them.
- Represent UI states explicitly: loading, empty, error, success, optimistic, disabled.
- Use accessibility-first markup and interaction patterns rather than retrofitting later.

## Verification

- Test rendered behavior and user interactions, not component internals.
- Verify keyboard access, focus behavior, and real loading/error states.
- Watch rerenders and bundle cost, but optimize from measurement instead of reflexively wrapping everything in `memo`.
- Keep storybook/demo states or equivalent examples for reusable UI pieces when the project benefits from them.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Effects that mirror props into state or trigger request chains with unclear ownership.
- Global state for ephemeral local UI concerns.
- One component that fetches data, owns forms, renders layout, and handles every button on the page.
- Custom hooks that hide complicated side effects behind cute names and no contract.
