# Maestro Marketplace

## Overview

The Maestro Marketplace is the official distribution channel for Maestro plugins and extensions. It provides a centralized repository where users can discover, install, and manage Maestro-compatible plugins.

## Marketplace Structure

### Plugin Registry

Each plugin in the marketplace is defined by a `plugin.json` file in its repository root. This file contains:

- **Plugin Metadata**: Name, description, version, author, license
- **Platform Support**: Claude Code and/or OpenCode compatibility
- **Installation Commands**: One-line installers for each platform
- **Feature Lists**: Capabilities and functionality
- **Dependencies**: Required Python packages and versions
- **Configuration Paths**: Global and project config locations
- **Integration Points**: Hooks, commands, agents, skills

### Marketplace Index

The marketplace index is maintained at:
```
https://github.com/scooter-lacroix/maestro-marketplace
```

This repository contains:
- `plugins/` - Plugin registry entries
- `index.json` - Complete plugin catalog
- `featured.json` - Curated plugin selections
- `categories.json` - Plugin categories and tags

## Plugin Submission

### Requirements

To submit a plugin to the Maestro Marketplace:

1. **plugin.json**: Complete metadata file in repository root
2. **Installation**: One-line installer for Claude Code (`install-claude-code.sh`)
3. **OpenCode Support** (Optional): One-line installer for OpenCode (`install-opencode.sh`)
4. **Documentation**: Comprehensive README with usage examples
5. **License**: Open-source license (MIT, Apache-2.0, GPL-3.0, etc.)
6. **Tests**: Test suite demonstrating functionality
7. **Versioning**: Semantic versioning (MAJOR.MINOR.PATCH)

### Submission Process

1. **Fork** the marketplace repository
2. **Add** your plugin entry to `plugins/your-plugin-name.json`
3. **Test** installation with provided one-line installers
4. **Submit** pull request to marketplace repository
5. **Review**: Marketplace maintainers will review your submission
6. **Publish**: Once approved, your plugin appears in the marketplace

### Plugin Entry Format

```json
{
  "name": "your-plugin",
  "repository": "https://github.com/yourusername/your-plugin",
  "version": "1.0.0",
  "description": "Brief description of your plugin",
  "author": "your-username",
  "license": "MIT",
  "claude_code": {
    "supported": true,
    "min_version": "1.0.0",
    "install_command": "curl -sSL https://raw.githubusercontent.com/yourusername/your-plugin/main/install-claude-code.sh | bash"
  },
  "opencode": {
    "supported": false
  },
  "categories": ["Development", "Tools"],
  "keywords": ["development", "tools"]
}
```

## Installation

### From Marketplace (Coming Soon)

```bash
# List available plugins
maestro marketplace list

# Search for plugins
maestro marketplace search <query>

# Install a plugin
maestro marketplace install <plugin-name>

# Update a plugin
maestro marketplace update <plugin-name>

# Remove a plugin
maestro marketplace remove <plugin-name>
```

### Manual Installation

For plugins not yet in the marketplace:

```bash
# Claude Code
curl -sSL https://raw.githubusercontent.com/author/plugin/main/install-claude-code.sh | bash

# OpenCode
curl -sSL https://raw.githubusercontent.com/author/plugin/main/install-opencode.sh | bash
```

## Marketplace CLI Commands

### List Plugins

```bash
/maestro marketplace list
```

Shows all available plugins with:
- Plugin name and description
- Version and author
- Installation status
- Category and tags

### Search Plugins

```bash
/maestro marketplace search <query>
```

Search by:
- Plugin name
- Description keywords
- Tags
- Author
- Category

### Install Plugin

```bash
/maestro marketplace install <plugin-name>
```

Installs plugin by:
1. Fetching plugin metadata from marketplace
2. Running one-line installer
3. Verifying installation
4. Registering plugin in local registry

### Update Plugin

```bash
/maestro marketplace update <plugin-name>
```

Updates installed plugin to latest version.

### Remove Plugin

```bash
/maestro marketplace remove <plugin-name>
```

Removes plugin from system while preserving configuration.

## Configuration

### Marketplace Settings

Configure marketplace behavior in `~/.claude/maestro.local.md`:

```yaml
marketplace:
  enabled: true
  auto_update: true
  update_interval: "7d"
  sources:
    - https://github.com/scooter-lacroix/maestro-marketplace
    - https://your-custom-marketplace.com
```

### Custom Marketplace Sources

You can add custom marketplace sources:

```yaml
marketplace:
  sources:
    - name: "official"
      url: "https://github.com/scooter-lacroix/maestro-marketplace"
    - name: "community"
      url: "https://github.com/community/maestro-plugins"
    - name: "private"
      url: "https://your-private-marketplace.com"
      auth_token: "${MARKETPLACE_TOKEN}"
```

## Plugin Development

### Creating a Plugin

1. **Initialize Plugin Structure**:
   ```bash
   mkdir my-maestro-plugin
   cd my-maestro-plugin
   ```

2. **Create plugin.json**:
   ```json
   {
     "name": "my-plugin",
     "version": "1.0.0",
     "description": "My awesome Maestro plugin",
     "claude_code": {
       "supported": true,
       "commands": ["myplugin:command"]
     }
   }
   ```

3. **Create Installer** (`install-claude-code.sh`):
   ```bash
   #!/bin/bash
   PLUGIN_DIR="$HOME/.claude/plugins/my-plugin"
   mkdir -p "$PLUGIN_DIR"
   cp -r . "$PLUGIN_DIR/"
   echo "Plugin installed to $PLUGIN_DIR"
   ```

4. **Create Commands** (`claude-code/commands/myplugin:command.md`):
   ```markdown
   ---
   description: My awesome command
   ---

   # My Command

   This command does awesome things.
   ```

5. **Test Locally**:
   ```bash
   ./install-claude-code.sh
   ```

6. **Submit to Marketplace**:
   - Fork marketplace repository
   - Add plugin entry
   - Submit pull request

### Plugin Best Practices

- **One-Line Installers**: Make installation painless
- **Clear Descriptions**: Explain what your plugin does
- **Version Management**: Use semantic versioning
- **Documentation**: Comprehensive README with examples
- **Testing**: Include test suite
- **Error Handling**: Graceful failure modes
- **Updates**: Maintain changelog
- **Support**: Provide issue tracker

## Security

### Plugin Verification

Plugins in the official marketplace are verified for:
- **Code Safety**: No malicious code patterns
- **Dependencies**: Trusted package sources
- **Installation**: Safe installer scripts
- **Updates**: Verified version control

### Security Best Practices

1. **Only install from trusted sources**
2. **Review installer scripts** before running
3. **Keep plugins updated**
4. **Report security issues** to maintainers
5. **Use plugin sandboxing** when available

## Troubleshooting

### Installation Fails

```bash
# Check installer permissions
chmod +x install-claude-code.sh

# Run with debug output
bash -x install-claude-code.sh
```

### Plugin Not Found

```bash
# Update marketplace index
/maestro marketplace refresh

# Check plugin name spelling
/maestro marketplace search <partial-name>
```

### Version Conflicts

```bash
# Check installed version
/maestro marketplace info <plugin-name>

# Force reinstall
/maestro marketplace reinstall <plugin-name>
```

## Contributing

Contributions to the marketplace are welcome:

- **New Plugins**: Submit your own plugins
- **Bug Reports**: Report issues with existing plugins
- **Documentation**: Improve documentation
- **Reviews**: Help review plugin submissions

## License

The Maestro Marketplace is licensed under the MIT License. See LICENSE for details.

## Support

- **Issues**: https://github.com/scooter-lacroix/Maestro/issues
- **Discussions**: https://github.com/scooter-lacroix/Maestro/discussions
- **Documentation**: https://github.com/scooter-lacroix/Maestro/blob/main/docs

---

**Last Updated**: 2026-01-04
**Marketplace Version**: 1.0.0
