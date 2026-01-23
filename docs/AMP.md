# Maestro for Amp CLI

Complete guide to using Maestro with Sourcegraph's Amp CLI.

## What is Maestro for Amp?

Maestro is a spec-driven development framework that integrates with Amp CLI's MCP ecosystem, providing structured project planning, track-based development, and access to LeIndex code analysis capabilities.

## Installation

See [Installation Guide](INSTALLATION.md) for complete setup instructions.

Quick install:
```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash
```

In the Conductor Wizard, ensure **Amp CLI** is enabled.

## What Gets Installed

- **MCP Configuration**: LeIndex MCP server entry in `~/.config/amp/settings.json` under `amp.mcpServers`

## Amp CLI Integration Notes

Amp CLI is primarily an MCP (Model Context Protocol) tool. Unlike other integrations, Maestro does not install custom commands for Amp CLI. Instead, it provides:

1. **LeIndex MCP Server**: Enables 5-layer code analysis within Amp
2. **MCP Tool Bridge**: Access to Maestro's analysis capabilities via Amp's MCP interface

## Using Maestro with Amp CLI

### LeIndex MCP Server

Maestro installs the LeIndex MCP server in your Amp configuration:

```json
{
  "amp.mcpServers": {
    "leindex": {
      "command": "maestro",
      "args": ["mcp", "proxy", "leindex"],
      "env": {}
    }
  }
}
```

### LeIndex Capabilities

When enabled, LeIndex provides:

- **5-layer code analysis**: AST, call graph, CFG, DFG, program slicing
- **Multi-language support**: Python, JavaScript/TypeScript, Rust, Go, Java, C/C++
- **Token-efficient output**: Ultra mode for LLM consumption
- **Context bundles**: Optimized context for orchestration loops

### Available MCP Tools

Once integrated, the following LeIndex tools are available via Amp:

| Tool | Description |
|------|-------------|
| `leindex_analyze` | Analyze source code files |
| `leindex_phase1` | Token-efficient AST analysis |
| `leindex_phase2` | Call graph analysis |
| `leindex_phase3` | Control flow graph analysis |
| `leindex_phase4` | Data flow graph analysis |
| `leindex_phase5` | Program slicing analysis |
| `leindex_context` | Generate context bundle for orchestration |

### Using LeIndex via Amp CLI

Since Amp CLI is MCP-focused, you'll interact with LeIndex through Amp's MCP interface:

```bash
# Start Amp CLI with LeIndex available
amp

# Within Amp, access LeIndex tools via MCP
# (specific commands depend on Amp's MCP interface)
```

## Troubleshooting

### "LeIndex not available in Amp"
**Solution**: Check MCP configuration
```bash
cat ~/.config/amp/settings.json | grep -A5 "amp.mcpServers"
```

### "Settings file not found"
**Solution**: Ensure Amp config directory exists
```bash
mkdir -p ~/.config/amp
```

### "MCP server not starting"
**Solution**: Verify maestro binary is in PATH
```bash
which maestro
# Should return: /home/user/.local/bin/maestro
```

## Maestro Workflow with Amp CLI

While Amp CLI doesn't support custom commands like other tools, you can still use Maestro's core workflow:

### 1. Use Maestro CLI Directly

```bash
# Initialize your project
maestro setup

# Create a track
maestro newTrack "Add ETL pipeline for data processing"

# Implement the track
maestro implement etl-pipeline
```

### 2. Use Amp CLI for Code Analysis

After setting up your project with Maestro, use Amp CLI with LeIndex for analysis:

```bash
# Start Amp
amp

# Use LeIndex tools via Amp's MCP interface
# (access depends on Amp's specific MCP tool syntax)
```

## Integration Example: ETL Pipeline

```bash
# 1. Plan with Maestro
mkdir etl-project && cd etl-project
maestro setup
maestro newTrack "Build data validation pipeline"

# 2. Implement with Maestro (amp-code agent selected automatically)
maestro implement data-validation

# 3. Analyze with Amp CLI + LeIndex
amp
# Use LeIndex MCP tools to analyze the implemented code
```

## Best Practices

1. **Use Maestro for planning** - Structured specs and plans
2. **Use Amp for analysis** - LeIndex-powered code insights via MCP
3. **Leverage amp-code agent** - For ETL/data pipeline tasks
4. **Combine workflows** - Maestro for orchestration, Amp for analysis
5. **Check MCP status** - Verify LeIndex is available in Amp

## Architecture

```
┌─────────────┐         ┌──────────────┐
│   Maestro   │         │  Amp CLI     │
│  (CLI/Bin)  │         │   (MCP)      │
└──────┬──────┘         └──────┬───────┘
       │                       │
       │     ┌─────────────────┘
       │     │
       ▼     ▼
┌─────────────────────┐
│   LeIndex MCP       │
│   Server (stdio)    │
│                     │
│ • 5-layer analysis  │
│ • Multi-language    │
│ • Token-efficient   │
└─────────────────────┘
```

## See Also

- [Claude Code Guide](CLAUDE-CODE.md) - Using Maestro with Claude Code
- [OpenCode Guide](OPENCODE.md) - Using Maestro with OpenCode
- [LeIndex Documentation](../maestro/leindex/docs/) - Deep dive into LeIndex capabilities
