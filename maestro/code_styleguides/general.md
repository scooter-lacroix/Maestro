# General Code Style Guide

## Principles

- Prefer correct, complete implementations over minimal ones
- Use appropriate data structures and algorithms
- Fix root causes, not symptoms
- Include necessary error handling and validation at system boundaries
- No stub, mock, or simulated implementations in production code (test doubles are acceptable for testing)

## TypeScript/React

- Use functional components with hooks
- Prefer `interface` for object types, `type` for unions/intersections
- Use `const` by default, `let` only when reassignment needed
- Named exports preferred over default exports
- Use template literals over string concatenation

## Rust

- Follow `rustfmt` and `clippy` recommendations
- Use `Result<T, E>` for recoverable errors, `panic!` only for programmer errors
- Prefer `&str` over `String` in function parameters
- Use `#[derive]` for standard traits where possible
- Document public APIs with `///` doc comments

## General

- 2-space indentation for TypeScript/React, 4-space for Rust
- Max line length: 120 characters
- Descriptive variable and function names
- No commented-out code in committed files
