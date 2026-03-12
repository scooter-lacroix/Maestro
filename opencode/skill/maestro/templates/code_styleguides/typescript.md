# TypeScript Guide

When I write TypeScript, I use the type system to make invalid states harder to represent and refactors safer to perform.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Precise types at boundaries and simple runtime code in the middle.
- Small modules with explicit contracts.
- A strict compiler configuration that catches drift early.
- Readable code that does not force the type system to become a puzzle.

## Required defaults

- Turn on strict mode and keep it on. If the codebase can support `noUncheckedIndexedAccess` and related strict flags, I prefer them too.
- Use `type` aliases by default for unions and data shapes; use `interface` when open extension or declaration merging is actually useful.
- Prefer discriminated unions, branded types, or narrow enums over piles of booleans and string literals drifting through the codebase.
- Treat `unknown` as the right starting point for untrusted input and validate it before narrowing.
- Use `const` heavily, keep functions small, and make return types explicit when they clarify the contract.

## Architecture

- Validate requests, environment values, storage records, and third-party payloads at the boundary so the core can trust its inputs.
- Keep domain types separate from transport types when external shape and internal meaning differ.
- Prefer plain functions and focused modules over class-heavy design unless lifecycle or polymorphism is central to the problem.
- Model state transitions with explicit result types instead of `null` plus comment conventions.

## Verification

- Test runtime behavior, not just type behavior; the compiler cannot prove external inputs are honest.
- Use linting and formatting so review can focus on logic and boundaries.
- Watch compile-time complexity in over-generic code; a perfect type that nobody can maintain is not a win.
- Keep build-time and runtime module boundaries aligned so imports behave predictably in tooling and production.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- `any`, broad type assertions, and non-null assertions used as routine escape hatches.
- Generic utility types that make ordinary data flow impossible to read.
- Default exports in large codebases when named exports make navigation easier.
- Runtime logic that depends on compile-time types without doing real validation.
