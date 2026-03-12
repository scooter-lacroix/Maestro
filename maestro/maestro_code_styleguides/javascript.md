# JavaScript Guide

When I write JavaScript, I keep the language sharp edges contained with clear module boundaries, simple data flow, and runtime checks at the edges.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Readable code that survives JavaScript's dynamic nature.
- Modern ESM modules and explicit dependencies.
- Small functions and predictable async flow.
- A migration path to stricter typing when the codebase wants it.

## Required defaults

- Use `const` by default and `let` only when reassignment is real and local.
- Prefer ESM, named exports, and small focused modules.
- Use early returns and explicit guards instead of deeply nested conditionals.
- Validate external input at the boundary because JavaScript will not save me later.
- Use JSDoc for public modules in JS-only codebases when it helps readers and tooling.

## Architecture

- Favor plain objects and functions over class-heavy design unless long-lived stateful objects are genuinely the right model.
- Keep domain logic separate from HTTP, DOM, storage, or framework glue.
- Represent async workflows with `async`/`await` unless a stream or event model is actually the domain.
- Name modules for the feature or responsibility they own, not for generic reuse aspirations.

## Verification

- Test behavior through public functions and module boundaries.
- Cover failure paths, undefined/null input, and partial external failures because those are where dynamic code breaks.
- Use linting and formatting so style questions do not dominate review.
- Watch for accidental event-loop blocking in CPU-heavy or collection-heavy code.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- `var`, implicit globals, and mutation shared across distant modules.
- Promise chains when `async`/`await` makes the story clearer.
- Monkey-patching built-ins, magical metaprogramming, and clever proxies in business code.
- Stringly typed protocols with no validation or normalization step.
