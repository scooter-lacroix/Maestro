# C++ Guide

When I write C++, I use modern language features to reduce footguns, not to show off the language.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Value semantics, RAII, and strong ownership boundaries.
- APIs that make misuse difficult and invariants explicit.
- Readable modern C++ over template gymnastics and inheritance webs.
- Performance guided by profiling, not folklore.

## Required defaults

- Use the project formatter, prefer C++20 or the newest supported standard, and compile with strong warnings enabled.
- Default to stack allocation and standard containers; reach for raw `new` or `delete` only at interop edges.
- Use `std::unique_ptr` for unique ownership and `std::shared_ptr` only when shared lifetime is truly required.
- Prefer `std::string_view`, spans, and references for non-owning access, but only when the lifetime is unambiguous.
- Mark immutability with `const`, and use `enum class`, `constexpr`, and `[[nodiscard]]` when they strengthen the API.

## Architecture

- Model domain concepts with concrete types and small public interfaces.
- Prefer composition over inheritance; inherit for substitutability, not for code sharing.
- Keep templates narrow and purposeful. If a non-template design is simpler, I use it.
- Isolate platform, threading, and third-party interop behind clean boundaries.

## Verification

- Use unit tests for domain logic and focused integration tests for threading, filesystem, network, or ABI boundaries.
- Run sanitizers and static analysis in development or CI where possible.
- Make exception policy explicit. If the project uses exceptions, reserve them for exceptional cases; if it does not, return rich error types consistently.
- Benchmark hot paths before adding custom allocators, intrusive containers, or low-level concurrency tricks.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Manual memory management in ordinary application code.
- Overloaded operator tricks, implicit conversions, and surprising constructors.
- Wide inheritance trees, virtual-by-default design, and singleton-heavy architecture.
- Premature micro-optimizations that erase clarity without measured benefit.
