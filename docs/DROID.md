# Maestro for Droid CLI

Complete guide to using Maestro with Factory's Droid CLI.

## What is Maestro for Droid?

Maestro is a spec-driven development framework that integrates with Droid CLI's MCP ecosystem, providing structured project planning, track-based development, and access to LeIndex code analysis capabilities.

## Installation

See [Installation Guide](INSTALLATION.md) for complete setup instructions.

Quick install:
```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash
```

In the Conductor Wizard, ensure **Droid CLI** is enabled.

## What Gets Installed

- **MCP Configuration**: LeIndex MCP server entry in `~/.factory/mcp.json` with `type: "stdio"`

## Droid CLI Integration Notes

Droid CLI is primarily an MCP (Model Context Protocol) tool from Factory. Like Amp CLI, Maestro does not install custom commands for Droid CLI. Instead, it provides:

1. **LeIndex MCP Server**: Enables 5-layer code analysis within Droid
2. **MCP Tool Bridge**: Access to Maestro's analysis capabilities via Droid's MCP interface

## Using Maestro with Droid CLI

### LeIndex MCP Server

Maestro installs the LeIndex MCP server in your Droid configuration:

```json
{
  "mcpServers": {
    "leindex": {
      "type": "stdio",
      "command": "maestro",
      "args": ["mcp", "proxy", "leindex"]
    }
  }
}
```

**Note**: The `type: "stdio"` field is required by Droid's MCP configuration format.

### LeIndex Capabilities

When enabled, LeIndex provides:

- **5-layer code analysis**: AST, call graph, CFG, DFG, program slicing
- **Multi-language support**: Python, JavaScript/TypeScript, Rust, Go, Java, C/C++
- **Token-efficient output**: Ultra mode for LLM consumption
- **Context bundles**: Optimized context for orchestration loops

### Available MCP Tools

Once integrated, the following LeIndex tools are available via Droid:

| Tool | Description |
|------|-------------|
| `leindex_analyze` | Analyze source code files |
| `leindex_phase1` | Token-efficient AST analysis |
| `leindex_phase2` | Call graph analysis |
| `leindex_phase3` | Control flow graph analysis |
| `leindex_phase4` | Data flow graph analysis |
| `leindex_phase5` | Program slicing analysis |
| `leindex_context` | Generate context bundle for orchestration |

### Using LeIndex via Droid CLI

Since Droid CLI is MCP-focused, you'll interact with LeIndex through Droid's MCP interface:

```bash
# Start Droid CLI with LeIndex available
droid

# Within Droid, access LeIndex tools via MCP
# (specific commands depend on Droid's MCP interface)
```

<<<<<<< HEAD
## Pi-Mono Integration

Maestro v2.5 includes Pi-Mono integration for subagent workflows with adaptive model selection.

### Available Commands

| Command | Description |
|---------|-------------|
| `maestro pi-status` | Show Pi-Mono configuration |
| `maestro pi-test` | Test subagent functionality |
| `maestro pi-agents` | List available pi agents |

### Implementation Flags

When using `maestro implement`, you can specify Pi-Mono execution modes:

```bash
# Single agent execution
maestro implement my-track --pi-agent scout

# Chain execution (sequential)
maestro implement my-track --pi-chain scout,architect,critic

# Parallel execution
maestro implement my-track --pi-parallel scout,kraken
```

**Available Pi Agents**: `scout`, `architect`, `critic`, `kraken`

### Configuration

Pi-Mono settings are stored in: `~/.maestro/config/pi-mono.yaml`

=======
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
## Troubleshooting

### "LeIndex not available in Droid"
**Solution**: Check MCP configuration
```bash
cat ~/.factory/mcp.json | grep -A5 "leindex"
```

### "Missing 'type' field"
**Solution**: Ensure MCP config has `type: "stdio"`
```bash
# Verify config structure
cat ~/.factory/mcp.json | jq '.mcpServers.leindex'
```

### "Config file not found"
**Solution**: Ensure Factory config directory exists
```bash
mkdir -p ~/.factory
```

### "MCP server not starting"
**Solution**: Verify maestro binary is in PATH
```bash
which maestro
# Should return: /home/user/.local/bin/maestro
```

## Maestro Workflow with Droid CLI

While Droid CLI doesn't support custom commands like other tools, you can still use Maestro's core workflow:

### 1. Use Maestro CLI Directly

```bash
# Initialize your project
maestro setup

# Create a track
maestro newTrack "Add spec-driven workflow"

# Implement the track
maestro implement spec-workflow
```

### 2. Use Droid CLI for Code Analysis

After setting up your project with Maestro, use Droid CLI with LeIndex for analysis:

```bash
# Start Droid
droid

# Use LeIndex tools via Droid's MCP interface
# (access depends on Droid's specific MCP tool syntax)
```

## Integration Example: Spec-Driven Development

```bash
# 1. Plan with Maestro
mkdir spec-project && cd spec-project
maestro setup
maestro newTrack "Implement spec-driven CLI framework"

# 2. Implement with Maestro
maestro implement cli-framework

# 3. Analyze with Droid CLI + LeIndex
droid
# Use LeIndex MCP tools to analyze the implemented code
```

## Factory Integration

Maestro's integration with Droid CLI is particularly useful when working with Factory's spec-driven development workflow:

1. **Spec Creation**: Use Maestro to create comprehensive specifications
2. **Implementation**: Use Maestro's track-based implementation
3. **Validation**: Use Droid + LeIndex for code analysis and validation
4. **Iteration**: Loop back through Maestro for refinements

## Best Practices

1. **Use Maestro for planning** - Structured specs and plans
2. **Use Droid for analysis** - LeIndex-powered code insights via MCP
3. **Follow Factory workflow** - Spec-driven development practices
4. **Combine workflows** - Maestro for orchestration, Droid for analysis
5. **Check MCP status** - Verify LeIndex is available in Droid

## Architecture

```
┌─────────────┐         ┌──────────────┐
│   Maestro   │         │  Droid CLI   │
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

## Comparison: Droid vs Other Tools

| Feature | Droid CLI | Claude Code | OpenCode |
|---------|-----------|-------------|----------|
| Custom Commands | No (MCP only) | Yes | Yes |
| MCP Integration | Yes (stdio) | Yes | Yes |
| Skill/Plugin System | No | Yes | Yes |
| LeIndex Support | Yes (MCP) | Yes (MCP) | Yes (MCP) |
| Spec-Driven Workflow | Via Factory | Native | Native |

## See Also

- [Claude Code Guide](CLAUDE-CODE.md) - Using Maestro with Claude Code
- [OpenCode Guide](OPENCODE.md) - Using Maestro with OpenCode
- [Factory Documentation](https://factory.pub) - Spec-driven development platform
- [LeIndex Documentation](../maestro/leindex/docs/) - Deep dive into LeIndex capabilities
