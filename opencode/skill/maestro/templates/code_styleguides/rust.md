# Rust Guide

When I write Rust, I use the type system, ownership model, and module boundaries to make bad states hard to express and safe code the easy path.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- APIs that encode invariants in types instead of comments.
- Clear ownership and lifetime boundaries without unnecessary cleverness.
- Small modules with explicit visibility.
- Reliability first, then performance guided by measurement.

## Required defaults

- Use `rustfmt` and `clippy`; style should be automated.
- Prefer borrowing over cloning, but do not contort code to avoid every clone at the expense of clarity.
- Use `Result` and `Option` idiomatically and add context to errors at boundaries that matter.
- Prefer newtypes, enums, and small structs to represent domain meaning precisely.
- Keep `pub` surfaces narrow and internal helpers private by default.

## Architecture

- Separate pure domain logic from IO, parsing, async runtimes, and external systems.
- Use traits for behavior abstraction where multiple implementations or test seams are real, not speculative.
- Model state machines with enums and pattern matching rather than boolean flag combinations.
- Make async ownership, cancellation, and backpressure rules explicit in service code.

## Verification

- Write unit tests close to pure logic and integration tests around crates or runtime boundaries.
- Use `unwrap` or `expect` only in tests, prototypes, or places where a documented invariant truly makes failure unrecoverable.
- Benchmark hot paths before reaching for unsafe code or exotic data structures.
- If unsafe code is necessary, isolate it, document the invariants, and keep the surface area tiny.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Large modules exporting everything because visibility felt inconvenient.
- Trait hierarchies and generic abstraction that make the code harder to read than the problem demands.
- Blind cloning, panicking in library code, or stringly typed domain values.
- Unsafe code used as a shortcut for design discipline.
