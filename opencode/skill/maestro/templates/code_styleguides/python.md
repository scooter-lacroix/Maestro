# Python Guide

When I write Python, I aim for explicit data shapes, small modules, and code that stays readable even after the third feature lands.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Readability, predictable behavior, and simple composition.
- Type hints where they clarify contracts and keep refactors safe.
- Clear boundary handling around files, networks, databases, and CLIs.
- A codebase that still feels simple once scripts grow into systems.

## Required defaults

- Use a formatter and linter (`ruff`/`black` or project equivalents) so style stays automatic.
- Annotate public functions, core domain models, and boundary objects with type hints.
- Prefer `pathlib`, context managers, and standard-library tools before adding dependencies.
- Use dataclasses or typed models for structured data instead of passing loose dicts everywhere.
- Write functions that do one thing and return one clear shape.

## Architecture

- Keep scripts with a clean `main()` and move reusable logic into modules that do not depend on CLI globals.
- Validate and normalize external input early, then keep downstream code working with trusted objects.
- Use exceptions intentionally: raise specific errors with context and catch them near boundaries that can respond.
- Split orchestration from business rules so tests can exercise behavior without filesystem or network setup.

## Verification

- Prefer fast unit tests for logic and targeted integration tests for real IO.
- Test unhappy paths: malformed input, timeouts, partial writes, and absent files are common Python failure modes.
- Watch import-time side effects, startup cost, and hidden global state in larger apps.
- Keep dependency lists lean and pinned deliberately when shipping production systems.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Mutable default arguments, bare `except`, and hidden module-level state.
- Classes that exist only to hold one function's worth of behavior.
- Passing around untyped nested dicts as if they were domain models.
- Metaprogramming or dynamic tricks when plain Python would be clearer.
