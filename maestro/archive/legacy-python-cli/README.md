# Legacy Python CLI

This directory contains the historical Python CLI implementation (`cli.py`) that was used before Maestro transitioned to a Rust-only architecture.

## Status

**ARCHIVED - Historical Reference Only**

This code is no longer used or maintained. All Maestro functionality has been ported to Rust.

## Migration

- The `maestro` binary is now built from Rust source using Cargo
- All CLI commands (tui, analyze, implement, memory, mcp) are in Rust
- See `/docs/adr/001-cli-ownership-and-binary-naming.md` for details

## DO NOT USE

This code is kept for reference purposes only. Do not use or modify this code.
