# Track: Cockpit Comprehensive Remediation

**Track ID:** cockpit-remediation_20260225  
**Created:** 2026-02-25  
**Status:** IN_PROGRESS  
**Priority:** P0–P4  
**Estimated Complexity:** Very High (35+ files, 5 major subsystems)

---

## Goal

Execute the comprehensive remediation plan covering 11 major work areas across the Maestro Cockpit TUI, MaesterClaw tab, Conductor, Settings, and Memory subsystems.

## Scope

### P0 — Critical Fixes
1. **Settings Save Button**: Fix silent error swallowing in `app.rs` and `Config::save()` returning `Ok(())` when `config_dir()` is `None`. Add toast feedback.
2. **Compilation Warnings**: Fix ~16 `dead_code` warnings in maestro-claw providers and 2 unused variable warnings in gateway/ws.rs.

### P1 — High Priority
3. **Tzar Review Remediation**: Fix IMP-2 through IMP-5, OPT-1/OPT-2, EDGE-1 through EDGE-4 from the tzar review.
4. **MaesterClaw Keyboard Shortcuts**: Wire all 12+ displayed shortcuts (N/E/D/T/R, A/C/X/R, P/W/D) to actual handlers. Create keybinding module.

### P2 — Medium Priority
5. **Agent Integration Panel**: Add `CapabilitiesSection::Agents` with detection, status, and launch capabilities.
6. **MaesterClaw Settings**: Extend `Config` with agent defaults, autonomy level, memory banking, conductor poll interval.
7. **Conductor Telemetry Bus**: Wire the global `BUS` in `telemetry.rs` to actually broadcast/subscribe events.

### P3 — Feature Work
8. **Memory Banking Integration**: Auto-bank at 8 required intervals via `AgentMemoryBridge` and `MemoryAutoBank`.
9. **Memory Tree Dependency Graph**: Tree view with expand/collapse, grouped by project/track/session.
10. **Interval-Based Memory Saving**: `MemoryTrigger` enum, mpsc channels, auto-bank service.

## Dependencies
- §8 Settings Save → None
- §1 Keyboard Shortcuts → None  
- §3 Agent Integration → §1
- §9 Memory Banking → §7 Conductor Telemetry, §11 Interval Saving

## Success Criteria
- `cargo check --workspace` — zero errors
- `cargo clippy --workspace --all-targets` — zero new warnings
- `cargo test --workspace` — all tests pass
- All 12+ MaesterClaw shortcuts functional
- Memory auto-banking operational at all 8 trigger points
