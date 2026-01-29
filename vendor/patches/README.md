# Vendor patch workflow

This directory contains patch files applied to vendored dependencies.

## Why

We vendor `libsql` under `vendor/libsql` (see `Cargo.toml` `[patch.crates-io]`).
Some warnings are fixed locally to keep builds clean. These patches must be
re-applied when `vendor/libsql` is updated.

## How it works

1. Patches live in this directory (e.g. `libsql-warning-fixes.patch`).
2. `scripts/apply-vendor-patches.sh` applies all patches in this directory.
3. The Makefile `build`/`dev-build` targets call the script automatically.

## Updating libsql

1. Update `vendor/libsql` to the new version.
2. Run:

```bash
scripts/apply-vendor-patches.sh
```

3. If a patch fails, update the patch file accordingly and re-run.
