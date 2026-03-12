# Ruby Guide

When I write Ruby, I want code that reads naturally without leaning on meta-programming or framework magic to do the reader's job.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Readable object design and expressive domain code.
- Small classes and modules with one reason to change.
- Clear side-effect boundaries around persistence, HTTP, jobs, and files.
- Pragmatic use of Ruby expressiveness without hiding control flow.

## Required defaults

- Prefer keyword arguments when a method takes several related options.
- Keep public method surfaces small and make return values predictable.
- Use plain objects for business workflows instead of pushing everything into models or controllers.
- Favor immutable or append-only data flow where practical, even in a mutable language.
- Let RuboCop or project tooling enforce formatting and common lint rules.

## Architecture

- Keep framework concerns at the edge and domain behavior in POROs or focused modules.
- Use modules for namespacing or shared behavior, not to create maze-like concern stacks.
- Make transactional boundaries explicit when a workflow crosses multiple writes or side effects.
- Prefer simple composition over callbacks and implicit lifecycle hooks whenever the behavior matters.

## Verification

- Test business objects directly and keep controller/request specs focused on boundary behavior.
- Watch query count, object churn, and background job idempotency in production-critical paths.
- Name tests by behavior, not by implementation detail.
- Log significant workflow decisions with enough context to debug live issues quickly.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Callback chains that make writes and side effects hard to trace.
- Meta-programming that hides behavior from search, tooling, or ordinary readers.
- Fat models and fat controllers that absorb every new feature.
- Monkey patches outside carefully isolated compatibility layers.
