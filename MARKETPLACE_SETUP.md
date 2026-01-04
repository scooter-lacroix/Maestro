# Maestro Marketplace Setup Guide

## Overview

This document describes the setup process for publishing Maestro to the Claude Code plugin marketplace, enabling users to install it with:

```bash
/plugin marketplace add scooterlacroix/maestro
/plugin install maestro
```

## Marketplace Structure

### Files Created/Modified

1. **plugin.json** - Enhanced with marketplace metadata
   - Added `displayName` field
   - Added `bugs` URL
   - Added `marketplace` section with repository info
   - Added `maestro:tui` command to the list
   - Additional keywords for better discoverability

2. **.claude-marketplace.json** - Marketplace manifest
   - Complete marketplace submission metadata
   - Platform-specific installation commands
   - Feature descriptions and dependencies
   - Screenshots and documentation links

3. **README.md** - Updated installation instructions
   - Primary installation method now uses `/plugin` commands
   - Manual installation moved to "Alternative" section
   - Clear distinction between marketplace and manual install

### Installation Flow

#### For Users (Marketplace)

```bash
# Step 1: Add marketplace repository
/plugin marketplace add scooterlacroix/maestro

# Step 2: Install plugin
/plugin install maestro

# Step 3: Run setup
/maestro:setup
```

#### For Users (Manual)

```bash
# Step 1: Run installer
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/main/install-claude-code.sh | bash

# Step 2: Run setup
/maestro:setup
```

## Plugin Structure Verification

### Required Commands

All commands are present in `/home/stan/Prod/maestro/claude-code/commands/`:

- ✅ `maestro:setup.md` - Initialize Maestro environment
- ✅ `maestro:newTrack.md` - Create new track
- ✅ `maestro:implement.md` - Implement track tasks
- ✅ `maestro:status.md` - View project progress
- ✅ `maestro:revert.md` - Revert work
- ✅ `maestro:configure.md` - Configure settings
- ✅ `maestro:tui.md` - Terminal UI interface
- ✅ `maestro:memory.md` - Memory commands
- ✅ `maestro:migrate:agent-deck.md` - Migration tools

### Required Templates

Templates are present in `/home/stan/Prod/maestro/claude-code/templates/`:

- ✅ `workflow.md` - Development workflow template
- ✅ `code_styleguides/` - Code style guide templates

### Installation Script

The installer at `/home/stan/Prod/maestro/install-claude-code.sh`:

1. Creates `~/.claude/commands/` directory
2. Copies all command files to commands directory
3. Creates `~/.claude/maestro-templates/` directory
4. Copies templates to templates directory
5. Provides clear installation feedback

## Marketplace Submission Checklist

### Plugin Requirements

- ✅ **plugin.json** - Complete metadata file in repository root
- ✅ **Installation** - One-line installer (`install-claude-code.sh`)
- ✅ **Documentation** - Comprehensive README with usage examples
- ✅ **License** - MIT License (see LICENSE file)
- ✅ **Tests** - Test suite demonstrating functionality (237 tests)
- ✅ **Versioning** - Semantic versioning (2.0.0)
- ✅ **Repository** - Public GitHub repository

### Marketplace Metadata

- ✅ Plugin name: `maestro`
- ✅ Display name: `Maestro - Unified Development Framework`
- ✅ Description: Clear and concise
- ✅ Tags/Keywords: Comprehensive list for discoverability
- ✅ Categories: Development, Testing, Workflow, Productivity
- ✅ Installation commands: Both marketplace and manual
- ✅ Platform support: Claude Code and OpenCode
- ✅ Commands list: All documented commands
- ✅ Features list: Key features highlighted
- ✅ Documentation links: Complete documentation URLs

### Documentation Requirements

- ✅ README.md with installation instructions
- ✅ Quick start guide
- ✅ Feature descriptions
- ✅ Command reference
- ✅ Examples and workflows
- ✅ Troubleshooting section
- ✅ Support links

## Marketplace Registration

### Option 1: Official Claude Code Marketplace

To register Maestro with the official Claude Code marketplace:

1. **Submit to Marketplace Registry**
   - Fork the official marketplace repository
   - Add plugin entry to `plugins/maestro.json`
   - Submit pull request

2. **Required Entry Format**

   ```json
   {
     "name": "maestro",
     "repository": "scooterlacroix/maestro",
     "version": "2.0.0",
     "description": "Spec-driven development with automatic agent selection and TDD enforcement",
     "author": "scooter-lacroix",
     "license": "MIT",
     "claude_code": {
       "supported": true,
       "min_version": "1.0.0",
       "install_command": "curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/main/install-claude-code.sh | bash"
     },
     "categories": ["Development", "Testing", "Workflow", "Productivity"],
     "keywords": ["development", "framework", "spec-driven", "tdd", "testing", "agent", "memory"]
   }
   ```

### Option 2: Self-Hosted Marketplace

For self-hosted marketplace installation:

1. **Create Marketplace Index**
   - Host a JSON index file at a public URL
   - Include Maestro plugin entry
   - Users add with: `/plugin marketplace add <your-index-url>`

2. **Index File Format**

   ```json
   {
     "version": "1.0.0",
     "plugins": [
       {
         "name": "maestro",
         "repository": "https://github.com/scooter-lacroix/Maestro",
         "version": "2.0.0",
         "install_command": "curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/main/install-claude-code.sh | bash"
       }
     ]
   }
   ```

## Verification Steps

### 1. Verify Plugin Structure

```bash
# Check plugin.json
cat /home/stan/Prod/maestro/plugin.json | jq .

# Check marketplace manifest
cat /home/stan/Prod/maestro/.claude-marketplace.json | jq .

# List commands
ls -la /home/stan/Prod/maestro/claude-code/commands/

# List templates
ls -la /home/stan/Prod/maestro/claude-code/templates/
```

### 2. Test Installation Script

```bash
# Test installer locally
bash -x install-claude-code.sh

# Verify installation
ls -la ~/.claude/commands/maestro*.md
ls -la ~/.claude/maestro-templates/
```

### 3. Verify README Installation Section

The README now shows:

```bash
# Marketplace Installation (Recommended)
/plugin marketplace add scooterlacroix/maestro
/plugin install maestro

# Alternative: Manual Installation
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/main/install-claude-code.sh | bash
```

### 4. Test Marketplace Commands

Once marketplace is available:

```bash
# Add repository
/plugin marketplace add scooterlacroix/maestro

# List plugins
/plugin marketplace list

# Install Maestro
/plugin install maestro

# Verify installation
/maestro:status
```

## Post-Installation Setup

After installation, users should:

1. **Run Setup Command**
   ```
   /maestro:setup
   ```

2. **Configure Settings** (Optional)
   ```
   /maestro:configure
   ```

3. **Create First Track**
   ```
   /maestro:newTrack Add user authentication
   ```

4. **Implement Track**
   ```
   /maestro:implement user-auth
   ```

## Maintenance

### Updating Plugin Version

1. Update `VERSION` file
2. Update `plugin.json` version field
3. Update `.claude-marketplace.json` version field
4. Update `pyproject.toml` version
5. Commit changes with tag: `git tag v2.0.1`
6. Push to GitHub: `git push --tags`

### Updating Marketplace Entry

When updating the marketplace entry:

1. Update version in marketplace registry
2. Update changelog
3. Release notes in GitHub release
4. Notify users of update availability

## Support

For issues or questions:

- **Documentation**: https://github.com/scooter-lacroix/Maestro/blob/main/docs
- **Issues**: https://github.com/scooter-lacroix/Maestro/issues
- **Discussions**: https://github.com/scooter-lacroix/Maestro/discussions

## Summary

Maestro is now ready for marketplace submission with:

- ✅ Enhanced `plugin.json` with marketplace metadata
- ✅ Complete `.claude-marketplace.json` manifest
- ✅ Updated `README.md` with marketplace installation commands
- ✅ All required commands and templates in place
- ✅ Installation script tested and working
- ✅ Comprehensive documentation

Users can install Maestro using:

```bash
/plugin marketplace add scooterlacroix/maestro
/plugin install maestro
```

Then run `/maestro:setup` to initialize their project.

---

**Status**: Ready for marketplace submission
**Last Updated**: 2026-01-04
**Plugin Version**: 2.0.0
