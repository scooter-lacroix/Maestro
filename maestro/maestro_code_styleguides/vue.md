# Vue Guide

When I write Vue, I keep components focused, reactivity intentional, and templates readable enough that the UI logic is obvious at a glance.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Small components with explicit props, emits, and state.
- Composition API code that is organized by feature behavior, not by framework ritual.
- Reactive state that is easy to track and hard to accidentally duplicate.
- Templates that stay declarative instead of becoming mini-programs.

## Required defaults

- Use the Composition API with `script setup` and TypeScript when the project supports it.
- Prefer `computed` for derived state and keep watchers for true side-effect synchronization.
- Type props and emits clearly so component contracts are visible and reliable.
- Keep refs and reactive objects narrow; a few focused atoms beat one giant mutable object.
- Co-locate composables with the feature they serve unless they are genuinely reusable across domains.

## Architecture

- Organize component code in the order readers care about: inputs, state, derived values, actions, effects.
- Keep store usage deliberate; not every sibling communication problem needs a global store.
- Separate data fetching, mutation, and formatting concerns so components do not become workflow god objects.
- Model UI states explicitly rather than overloading one boolean like `loading` to mean three things.

## Verification

- Test rendered behavior, emitted events, and key reactive transitions.
- Verify keyboard and focus behavior on interactive elements and modal-like flows.
- Watch watcher chains, unnecessary rerenders, and large reactive objects in complex screens.
- Keep async error handling visible so failures do not disappear into the console.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Business logic hidden in templates or watchers.
- Monolithic stores holding unrelated state with no clear ownership.
- Mixing Options API and Composition API patterns casually within the same feature.
- Template cleverness that saves five lines and costs everyone comprehension.
