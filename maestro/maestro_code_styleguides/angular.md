# Angular Guide

When I write Angular, I keep components lean, state explicit, and templates boring in the best possible way.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Standalone components and feature-local structure over sprawling NgModule-era indirection.
- Predictable change detection, explicit inputs and outputs, and minimal template logic.
- A small public API per component so parent-child relationships stay easy to reason about.
- Reactive data flow where streams or signals add clarity, not ceremony.

## Required defaults

- Use standalone components, `OnPush`, and typed forms by default.
- Keep templates declarative; move branching, mapping, and formatting into component code or pure pipes.
- Use signals for local UI state when they simplify the component, and RxJS for true stream composition or async workflows.
- Inject dependencies through constructors or `inject()` at the edge of the component or service, not deep in helper functions.
- Prefer `readonly` properties and immutable updates so change detection stays predictable.

## Architecture

- Organize by feature first, then by role inside the feature if needed.
- Use services for coordination with APIs, storage, or shared state; do not move simple component logic into services out of habit.
- Keep route resolvers and guards focused on access and fetch orchestration, not business rules.
- Represent view state explicitly: loading, empty, error, and ready should be distinct states.

## Verification

- Test components through their inputs, outputs, and rendered behavior rather than private methods.
- Unit test services with fake collaborators; integration test HTTP and router boundaries where behavior matters.
- Use Angular tooling and lint rules to enforce structure, not manual taste debates.
- Watch bundle size: lazy-load feature areas and keep shared dependencies intentional.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Business logic hidden in templates or lifecycle hooks chains.
- Subscription management scattered across the component instead of centralized patterns.
- Mutable shared state that updates from many places with no clear owner.
- RxJS for everything when simple computed state would be easier to maintain.
