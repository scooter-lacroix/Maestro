# Security Policy

## Supported Branches

Security fixes are expected on the actively maintained branch:

- `main`

Older branches may receive fixes at maintainer discretion, but `main` is the supported target.

## Reporting a Vulnerability

Please do not report security vulnerabilities in public issues or pull requests.

Instead:

1. Open a private GitHub security advisory for this repository, if available.
2. Include a clear description, impact, affected components, and reproduction steps.
3. If possible, include a minimal proof of concept and any suggested remediation.

## What to Include

A useful report should include:

- affected component or path
- impact and attack scenario
- required privileges or assumptions
- steps to reproduce
- whether the issue is configuration-specific or generally exploitable

## Response Goals

The project will aim to:

- acknowledge valid reports promptly
- reproduce and assess severity
- prepare a fix or mitigation
- credit reporters if they want public acknowledgement

## Scope Notes

This project includes:

- CLI tooling
- orchestration/runtime systems
- local and networked agent integrations
- TrackLens review surfaces
- MCP-related tooling and bridges

Vulnerabilities in any of those surfaces are in scope.
