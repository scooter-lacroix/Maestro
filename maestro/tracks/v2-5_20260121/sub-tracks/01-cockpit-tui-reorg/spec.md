# Sub-Track 01: Cockpit v2 TUI Re-Org & Distribution - Specification

## Objective

Make the Rust Cockpit v2 the canonical Maestro TUI (and the only supported one), and restructure the repository so the TUI is not nested under LeIndex internals.

## Requirements

### R1: Canonical Rust TUI Location

- The Rust Cockpit code must live in a dedicated module/crate representing the UI layer.
- The TUI must not be “buried” in `maestro/leindex/rust/src/cli/tui.rs`.

### R2: Single Entry Point

- `maestro tui` launches the Rust Cockpit.
- The Python CLI must not reference the Go TUI.

### R3: Retire Go TUI Wiring

- `Makefile`, docs, and any installer scripts must not reference `maestro/tui` (Go) or `maestro-tui` as a Go-built artifact.
- The Go TUI may remain in `maestro/archive/tui-go` as inert reference.

### R4: Installation / Distribution

- The installer must build and install required Rust binaries (Cockpit + LeIndex CLI surface).
- Cross-platform considerations must be explicitly handled (Linux/macOS/WSL at minimum).

## Acceptance Criteria

- `maestro tui` reliably starts the Rust Cockpit from a clean install.
- No build scripts, docs, or manifests require Go for the TUI.
- Repo structure clearly communicates that Cockpit v2 is the primary UI.

