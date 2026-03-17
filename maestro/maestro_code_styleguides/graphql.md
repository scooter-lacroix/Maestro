# GraphQL Guide

When I design GraphQL APIs, I treat the schema as a product contract, not a thin mirror of database tables.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Schemas that read like the domain language of the product.
- Stable contracts with deliberate evolution paths.
- Resolvers that stay thin and predictable.
- Performance characteristics I can explain before traffic exposes them.

## Required defaults

- Name fields and types for consumer meaning, not storage implementation.
- Prefer nullable fields only when absence is a legitimate business state; otherwise make constraints explicit with non-null types.
- Use input objects for anything beyond trivial mutations.
- Keep pagination, filtering, and ordering conventions consistent across the API.
- Expose one clear error strategy so clients are not guessing which failures appear in data versus errors.

## Architecture

- Resolvers orchestrate; services and domain code own business rules.
- Batch data access deliberately with tools like DataLoader or equivalent patterns when relation fan-out exists.
- Version by additive evolution and deprecation rather than forked schemas when possible.
- Design mutations around user intent, not generic CRUD verbs when the workflow has richer meaning.

## Verification

- Test schema behavior from the API boundary, including auth, nullability, and error cases.
- Watch field-level performance, N+1 patterns, and payload growth as part of normal API maintenance.
- Document cost-heavy operations and guard them with auth, complexity limits, or pagination requirements.
- Log resolver failures with enough context to identify the field path and root cause quickly.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Schemas that simply expose internal table structure and naming.
- Business logic buried in resolvers with copy-pasted authorization or validation.
- Inconsistent pagination or filtering semantics between sibling resources.
- Unlimited nested queries with no guardrails on cost.
