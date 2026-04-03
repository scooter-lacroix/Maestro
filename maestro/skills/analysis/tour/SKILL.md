---
name: tour
description: "Guided onboarding that introduces Maestro's capabilities with a warm, practical overview. Use when the user asks what you can do, wants to get started, asks for help navigating features, says show me around, or needs an overview of available capabilities."
user-invocable: false
---

# Tour

Provide a friendly onboarding experience when users ask about Maestro's capabilities.

## Trigger Conditions

Activate when the user's message matches phrases like:
- "what can you do?"
- "help me get started"
- "show me around"
- "what features are available?"

## Workflow

1. Detect onboarding intent from the user's message
2. Present a warm, categorized overview of capabilities
3. End with an invitation to start working

## Response Template

Present the following overview, adapting tone to be welcoming and practical:

### Code & Development
- **Write & edit code** across any language or framework
- **Debug issues** by tracing errors and finding root causes
- **Refactor** to improve structure without breaking behavior
- **Test** by writing and running tests to validate changes

### Memory & Context
- **Remember across sessions** with learnings persisted to PostgreSQL
- **Recall past work** by searching what worked or failed before
- **Handoffs** to create snapshots for resuming complex work later

### Research & Planning
- **Explore codebases** to understand unfamiliar projects quickly
- **Plan implementations** by architecting before coding
- **Search the web** for docs, solutions, and best practices

### Specialized Agents
Spawn sub-agents for complex tasks:
- `explorer` — map codebase structure
- `implementer` — implement with TDD workflow
- `debug` — investigate issues systematically

## Style Guidelines

- Be welcoming, not overwhelming
- Focus on practical value over exhaustive feature lists
- End with an open invitation: "What would you like to work on?"
- Highlight categories rather than listing every individual skill
