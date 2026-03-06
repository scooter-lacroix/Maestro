# Subtrack 07: Cleanup

## Objective
Remove non-functional wizard code from the codebase and update references.

## Requirements

### R1: Delete Files
- Delete `crates/cockpit/src/maesterclaw/readiness.rs` (useless wizard reducer)
- Remove MaesterClawSetupState from `state/types.rs`
- Remove MaesterClawSetupStep from `state/types.rs`
- Remove MaesterClawSetupCheck from `state/types.rs`

### R2: Update Imports
- Remove references to deleted code
- Update module exports in maesterclaw/mod.rs
- Fix any broken imports

### R3: No Regressions
- All tests still pass after cleanup
- cargo check --workspace succeeds
- No breaking changes to public APIs

## Acceptance Criteria
- [ ] readiness.rs deleted
- [ ] MaesterClawSetup* types removed from state/types.rs
- [ ] All imports updated
- [ ] All tests pass
- [ ] cargo check --workspace succeeds
