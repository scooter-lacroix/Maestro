# Svelte Guide

When I write Svelte, I use the framework's simplicity to keep components direct and expressive instead of hiding complexity behind clever reactivity tricks.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Small components with obvious data flow.
- Local state by default and shared stores only when ownership truly spans features.
- Accessible markup and interactions baked into the component, not patched in later.
- A codebase where reactivity stays easy to reason about.

## Required defaults

- Keep component scripts short and push reusable logic into focused modules or utilities.
- Use derived state instead of synchronizing duplicate state by hand.
- Prefer explicit props and events over hidden coupling through global stores.
- Keep styling local or token-driven so components remain portable and intentional.
- When using SvelteKit, keep server and client boundaries deliberate and validation close to actions or loaders.

## Architecture

- Group files by feature and let components own the UI they render.
- Represent async UI states explicitly rather than leaning on one generic 'loading' boolean for everything.
- Use stores for cross-cutting state with a clear owner and lifecycle, not as the default dumping ground.
- Prefer readable template code over packing logic into terse reactive declarations.

## Verification

- Test rendered behavior and key state transitions, not private implementation details.
- Verify keyboard, focus, and screen-size behavior on interactive components.
- Watch hydration and payload cost where SvelteKit routes mix server and client concerns.
- Keep demo or preview states for reusable UI where it pays off.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Large component files that own data fetching, business logic, and every presentational detail at once.
- Reactive statements that mutate several pieces of state in opaque ways.
- Global stores for one route's temporary UI state.
- Animation and transitions that fight usability or reduced-motion preferences.
