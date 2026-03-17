# Docker Guide

When I write Dockerfiles and Compose setups, I optimize for repeatable builds, small attack surface, and fast feedback in CI and local development.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Reproducible builds that behave the same on laptops and in CI.
- Minimal runtime images with only the files and tools the app needs.
- Layer ordering that keeps rebuilds fast.
- Secure-by-default containers with explicit runtime assumptions.

## Required defaults

- Use multi-stage builds for anything non-trivial.
- Pin base images intentionally and update them on purpose; avoid vague tags in production.
- Run as a non-root user whenever the workload allows it.
- Copy manifest files and install dependencies before copying the full source tree so caching works for me.
- Keep environment-specific values out of images and inject them at runtime.

## Architecture

- Treat the image as an artifact of the app, not as a place to improvise system administration.
- Use Compose for local orchestration and clarity, not to hide missing application startup discipline.
- Keep health checks meaningful: they should tell me if the service can actually serve, not just if the process exists.
- Separate build-only tools from runtime artifacts.

## Verification

- Build images in CI exactly the way production images are built.
- Scan images and dependency manifests as part of normal maintenance.
- Measure image size and startup cost if containers become slow or expensive to ship.
- Document required volumes, ports, env vars, and readiness assumptions close to the image definition.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Installing debugging packages into production images by default.
- Fat images that contain compilers, package managers, and source code they never use at runtime.
- Containers that depend on ad hoc shell startup scripts when the app can own startup cleanly.
- Using Docker to paper over poor local setup while production follows a different path.
