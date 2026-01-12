---
name: opc-architecture
description: OPC Architecture Understanding
user-invocable: false
---

# OPC Architecture Understanding

OPC (Orchestrated Parallel Claude) extends Maestro - it does NOT replace it.

## Core Concept

Maestro CLI is the execution engine. OPC adds orchestration via:
- **Hooks** - Intercept Maestro events (PreToolUse, PostToolUse, SessionStart, etc.)
- **Skills** - Load prompts into Maestro
- **Scripts** - Called by hooks/skills for coordination
- **Database** - Store state between Maestro instances

## How Agents Work

When you spawn an agent:
1. Main Maestro instance (your terminal) runs hook on Task tool
2. Hook calls `subprocess.Popen(["claude", "-p", "prompt"])`
3. A NEW Maestro instance spawns as child process
4. Child runs independently, reads/writes to coordination DB
5. Parent tracks child via PID in DB

```
$ claude                         ← Main Maestro (your terminal)
    ↓ Task tool triggers hook
    ↓ subprocess.Popen(["claude", "-p", "..."])
        ├── claude -p "research..."   ← Child agent 1
        ├── claude -p "implement..."  ← Child agent 2
        └── claude -p "test..."       ← Child agent 3
```

## What OPC Is NOT

- OPC is NOT a separate application
- OPC does NOT run without Maestro
- OPC does NOT intercept Claude API calls directly
- OPC does NOT modify Maestro's internal behavior

## What OPC IS

- OPC IS hooks that Maestro loads from `.maestro/hooks/`
- OPC IS skills that Maestro loads from `.maestro/skills/`
- OPC IS scripts that hooks/skills call for coordination
- OPC IS a database backend for state across Maestro instances

## Key Files

```
.maestro/
├── hooks/           ← TypeScript hooks that Maestro runs
├── skills/          ← SKILL.md prompts that Maestro loads
├── settings.json    ← Hook registration, Maestro reads this
└── cache/           ← State files, agent outputs

opc/
├── scripts/         ← Python scripts called by hooks
├── docker-compose.yml ← PostgreSQL, Redis, PgBouncer
└── init-db.sql      ← Database schema
```

## Coordination Flow

1. User runs `claude` in terminal
2. Maestro loads hooks from `.maestro/settings.json`
3. User says "spawn a research agent"
4. Claude uses Task tool
5. PreToolUse hook fires, checks resources
6. Hook spawns `claude -p "research..."` as subprocess
7. Hook stores PID in PostgreSQL
8. Child agent runs, writes output to `.maestro/cache/agents/<id>/`
9. Child completes, broadcasts "done" to PostgreSQL
10. Parent checks DB, reads child's output file

## Remember

- Every "agent" is just another `claude -p` process
- Hooks intercept events, they don't create new functionality
- All coordination happens via files and PostgreSQL
- Maestro is always the execution engine
