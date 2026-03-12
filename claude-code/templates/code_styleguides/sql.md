# SQL Guide

When I write SQL, I optimize for correctness first, then readability, then performance based on real query patterns.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Queries that clearly express intent and are safe to evolve.
- Schema design that reflects domain rules rather than punting them all to application code.
- Predictable query plans for important workloads.
- Migrations that are safe, reversible when possible, and operationally boring.

## Required defaults

- Name tables, columns, constraints, and indexes consistently and descriptively.
- Prefer explicit column lists over `SELECT *` in application queries and views.
- Use parameterized queries always; string-building SQL is not acceptable for application code.
- Reach for CTEs when they clarify the story, not just because they exist.
- Keep timestamps, nullability, and defaults deliberate and documented by the schema itself.

## Architecture

- Encode important integrity rules in constraints, keys, and indexes whenever the database can enforce them reliably.
- Design indexes from read and write patterns, not from guesswork or a blanket 'index everything' instinct.
- Treat migrations as code: additive first, backfill deliberately, remove old columns only after the transition is complete.
- Keep reporting or analytics queries separate from OLTP paths when they have different shape or cost profiles.

## Verification

- Test queries and migrations against realistic data volumes when performance matters.
- Inspect explain plans for hot queries and verify index usage after schema changes.
- Make data-changing operations explicit about transaction boundaries and lock implications.
- Monitor slow queries, deadlocks, and migration runtime as part of normal operations.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Hidden Cartesian joins, ambiguous ordering, and nullable columns with unclear meaning.
- Business-critical constraints enforced only in application code when the database can own them.
- One giant query that is technically clever but impossible to maintain.
- Long-running destructive migrations with no rollout or backfill plan.
