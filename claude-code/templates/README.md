# Maestro Templates

This directory contains templates used by Maestro for project setup.

## Files

### workflow.md
Default development workflow template that includes:
- Guiding principles
- Agent usage requirements (proactive automatic selection)
- Task workflow (TDD: Red → Green → Refactor)
- Fallback mechanisms

### code_styleguides/
Language-specific code style guides:
- `general.md` - General principles applying to all languages
- `go.md` - Go-specific conventions
- `html-css.md` - HTML/CSS best practices
- `javascript.md` - JavaScript conventions
- `python.md` - Python style guide
- `typescript.md` - TypeScript conventions

## Installation

These files are copied to `~/.claude/maestro-templates/` by the installer script.

## Usage

During `/maestro:setup`, users select which code style guides to include in their project. Selected guides are copied to `maestro/code_styleguides/` in the project directory.

## Customization

You can customize these templates for your organization:

1. Edit files in `~/.claude/maestro-templates/`
2. Changes will apply to future projects
3. Existing projects are not affected

## Documentation

See [../../docs/CLAUDE-CODE.md](../../docs/CLAUDE-CODE.md) for complete usage guide.
