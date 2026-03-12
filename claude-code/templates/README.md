# Maestro Templates

This directory contains templates used by Maestro for project setup.

## Files

### workflow.md
Default development workflow template that includes:
- Guiding principles
- Agent usage requirements (proactive automatic selection)
- Task workflow (TDD: Red -> Green -> Refactor)
- Fallback mechanisms

### code_styleguides/
A first-principles guide library written from Maestro's own coding defaults. The current set includes 27 guides across languages, frameworks, and adjacent technologies:

- Core: `general.md`, `docker.md`, `graphql.md`, `shell.md`, `sql.md`
- Systems: `c.md`, `cpp.md`, `go.md`, `rust.md`
- Application languages: `csharp.md`, `dart.md`, `java.md`, `javascript.md`, `kotlin.md`, `php.md`, `python.md`, `ruby.md`, `swift.md`, `typescript.md`
- Frontend and runtime stacks: `angular.md`, `html-css.md`, `nextjs.md`, `nodejs.md`, `react.md`, `svelte.md`, `threejs.md`, `vue.md`

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
