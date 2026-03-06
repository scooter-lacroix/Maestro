# MaestroClaw Functional Parity Matrix

This matrix compares the current `crates/maestro-claw` implementation against the three reference systems:

- ZeroClaw: `/mnt/WD-SSD/Prod/work_resources/zeroclaw`
- Moltis: `/mnt/WD-SSD/Prod/work_resources/moltis`
- IronClaw: `/mnt/WD-SSD/Prod/work_resources/ironclaw`

The assessment is intentionally concrete:

- `Present`: implemented in the active MaestroClaw runtime
- `Partial`: implemented, but thinner or less integrated than the references
- `Missing`: not present in the active runtime in a way that supports parity claims

## Matrix

| Capability | MaestroClaw | ZeroClaw | Moltis | IronClaw | Notes |
| --- | --- | --- | --- | --- | --- |
| CLI-first local agent harness | Present | Partial | Partial | Partial | MaestroClaw's strongest differentiator remains the local CLI-provider path in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/agent/cli_provider.rs` and the shared runtime path in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/agent/runtime.rs`. |
| Coherent tool-loop execution in live runtime | Present | Present | Present | Present | Gateway, channel, cron, and heartbeat surfaces now route through the shared agent loop instead of calling the provider directly: `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/gateway/mod.rs`, `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/channels/dispatcher.rs`, `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/cron/scheduler.rs`, `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/heartbeat/mod.rs`. |
| Onboarding / setup wizard | Partial | Present | Present | Present | MaestroClaw has `run_quick_setup` and `run_wizard` in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/onboard/mod.rs`, but the flow is much shallower than `/mnt/WD-SSD/Prod/work_resources/zeroclaw/src/onboard/wizard.rs` and `/mnt/WD-SSD/Prod/work_resources/ironclaw/src/setup/wizard.rs`. |
| Authenticated gateway surface | Present | Partial | Present | Present | MaestroClaw now enforces agent auth across REST, WebSocket, and SSE, and routes authenticated session execution through `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/agent_runtime.rs`, `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/routes.rs`, `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/ws.rs`, and `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/sse.rs`. It is still bearer-key based rather than passkey-based. |
| Channel coverage | Partial | Present | Present | Present | MaestroClaw has Telegram and Discord in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/channels/`, but not IronClaw's web/WASM channel model or ZeroClaw's broader operational channel surface. |
| Tool approvals in active runtime | Present | Present | Present | Present | MaestroClaw now has a live pending-approval workflow backed by `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/state.rs`, `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/agent.rs`, `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/agent_runtime.rs`, and `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/integration/security.rs`. |
| Sandboxing / runtime isolation | Partial | Present | Present | Present | MaestroClaw now enforces autonomy-aware file and shell restrictions through `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/agent/runtime.rs`, `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/tools/builtin/file.rs`, and `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/tools/builtin/shell.rs`, but it does not yet match the stronger integrated sandbox stacks in the references. |
| Cron / routines | Partial | Present | Present | Present | MaestroClaw has cron plus heartbeat in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/cron/` and `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/heartbeat/`, but not IronClaw's fuller routine/event model. |
| Skills system | Partial | Present | Present | Partial | MaestroClaw can manage local skills in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/skills/mod.rs`, but it is not yet as deep as ZeroClaw's broader integration model or Moltis's richer skills subsystem. |
| Cost / observability | Partial | Present | Present | Present | MaestroClaw now records runtime events and cost scaffolding in `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/cost/mod.rs` and `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/observability/mod.rs`, but the model is still lighter than the reference runtimes. |
| Extension / MCP / tool auth | Present | Partial | Partial | Present | MaestroClaw now exposes pending MCP auth requests, token submission, and authenticated MCP connection flow in `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/agent_runtime.rs`, `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/routes.rs`, `/mnt/WD-SSD/Prod/maestro/crates/gateway/src/ws.rs`, and `/mnt/WD-SSD/Prod/maestro/crates/core/src/capabilities/mcp.rs`. IronClaw still has the broader extension-management model. |

## Current Differentiators

MaestroClaw is already strongest where the references are relatively heavier:

- Local CLI-native operation without requiring API-key-first setup. The key surfaces are `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/agent/cli_provider.rs` and `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/onboard/mod.rs`.
- A coherent shared runtime path for gateway, channels, cron, and heartbeat via `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/agent/runtime.rs`.
- Autonomy-aware default tool registration that makes the live runtime safer by default while keeping the CLI-first developer workflow intact.

Those are not enough to claim overall superiority yet, but they are real foundations for a better local-first system than any one reference provides on its own.

## Priority Roadmap

The fastest defensible path to making MaestroClaw superior is:

1. Expand onboarding from "tool detection + defaults" into a true runtime bootstrap.
   Include autonomy policy, gateway secret generation, channel setup, and repair flows.
   Target surface: `/mnt/WD-SSD/Prod/maestro/crates/maestro-claw/src/onboard/mod.rs`.

2. Deepen gateway identity beyond bearer-key auth.
   Moltis is still the strongest reference here.
   Target gap: richer user/session identity, passkeys or equivalent, and tighter browser-facing auth ergonomics.

3. Deepen extension management beyond raw MCP auth.
   IronClaw is still the clearest reference pressure here.
   The goal is not feature parity alone; the goal is a better local-first extension lifecycle than IronClaw's more server-oriented model.

4. Deepen observability and cost accounting.
   The current scaffolding is useful, but superiority requires per-turn, per-tool, and per-surface visibility that is comparable to or better than ZeroClaw.

## Bottom Line

MaestroClaw is now in a better position to make parity claims around runtime coherence because the live surfaces no longer bypass the tool loop, the approval workflow is live, and authenticated MCP auth flow is present. Overall parity is still `partial`, but the remaining pressure is now mostly on onboarding depth, channel breadth, sandbox/runtime hardening, and richer identity/extension management rather than on missing core runtime features.
