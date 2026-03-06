# Subtrack 07: Cleanup - Plan

## Phase 1: Delete Non-Functional Code

### [ ] Task 1.1: Delete readiness.rs
- Delete `crates/cockpit/src/maesterclaw/readiness.rs`
- Verify file is removed

### [ ] Task 1.2: Remove MaesterClawSetupState
- Remove from `crates/cockpit/src/state/types.rs`
- Update any references

### [ ] Task 1.3: Remove MaesterClawSetupStep
- Remove from `crates/cockpit/src/state/types.rs`
- Update any references

### [ ] Task 1.4: Remove MaesterClawSetupCheck
- Remove from `crates/cockpit/src/state/types.rs`
- Update any references

## Phase 2: Update References

### [ ] Task 2.1: Update maesterclaw/mod.rs
- Remove readiness module export
- Verify other exports intact

### [ ] Task 2.2: Fix Broken Imports
- Search for references to deleted types
- Update or remove as needed

## Phase 3: Verification

### [ ] Task 3.1: Run Tests
- All existing tests pass

### [ ] Task 3.2: Workspace Check
- cargo check --workspace succeeds

### [ ] Task 3.3: Manual Verification
- [ ] Task: Maestro - User Manual Verification 'Subtrack 07: Cleanup'
