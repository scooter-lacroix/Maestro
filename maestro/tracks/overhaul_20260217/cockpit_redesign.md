# Maestro Cockpit TUI Redesign: MaesterClaw Design

## Executive Summary

This document outlines a comprehensive redesign of the Maestro Cockpit TUI using MaesterClaw design principles. The redesign addresses three key challenges:

1. **Welcome Screen** - First-time user onboarding
2. **Navigation Scalability** - Managing 10+ tabs without overcrowding
3. **Phase 3-5 Capability Presentation** - Cron jobs, MCP servers, sandbox, channels, web gateway

---

## 1. Design Philosophy: MaesterClaw Principles

### 1.1 Core Principles

**Boot Information Panel:**
- Clear status indicators for system state
- Model/provider visibility at startup
- Database connection status prominently displayed
- Feature flags and channel availability shown in single view
- ANSI-styled, polished terminal output

**Onboarding Wizard:**
- Step-by-step setup with progress indication (Step X of Y)
- Channel configuration with clear descriptions
- Sensible defaults with non-interactive quick setup option
- Visual checkmarks for completed steps
- "Next steps" guidance after setup completion

**Event-Driven Architecture:**
- Ordered streaming events with sequence tracking
- Real-time status updates
- Multi-panel layouts with clear focus indicators

### 1.2 MaesterClaw Design Tenets

1. **Transparency First** - Always show system state (model, database, tools, channels)
2. **Progressive Disclosure** - Start simple, reveal complexity as needed
3. **Contextual Help** - "?" key always available, context-sensitive
4. **Status at a Glance** - Single line header shows critical information
5. **Modal Overlays** - Complex operations use overlays, preserving context
6. **Keyboard Efficiency** - Power users navigate without leaving keyboard
7. **Visual Hierarchy** - Color, borders, and emphasis guide attention

---

## 2. Welcome Screen Design

### 2.1 First-Time Detection

The Cockpit should detect first-time users by checking for:
- `~/.maestro/.cockpit_initialized` marker file
- No existing sessions in the database
- No projects registered

### 2.2 Welcome Screen Layout

```
+==============================================================================+
|                                                                              |
|     ███████╗ █████╗  ██████╗ ██████╗ ██████╗ ███████╗                        |
|     ██╔════╝██╔══██╗██╔════╝██╔═══██╗██╔══██╗██╔════╝                        |
|     █████╗  ███████║██║     ██║   ██║██║  ██║█████╗                          |
|     ██╔══╝  ██╔══██║██║     ██║   ██║██║  ██║██╔══╝                          |
|     ██║     ██║  ██║╚██████╗╚██████╔╝██████╔╝███████╗                        |
|     ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝                        |
|                                                                              |
|     Autonomous Development Cockpit v2.5                                      |
|     Unified AI-Assisted Software Engineering                                |
|                                                                              |
+==============================================================================+
|                                                                              |
|  [Step 1 of 4] Workspace Setup                                               |
|                                                                              |
|  Maestro needs a workspace directory for projects and tracks.               |
|                                                                              |
|  Workspace path: [~/.maestro/workspace         ]                             |
|                                                                              |
|  [Tab: Next Field]  [Enter: Accept]  [Esc: Use Defaults]                    |
|                                                                              |
+==============================================================================+
```

### 2.3 Onboarding Wizard Steps

**Step 1: Workspace Setup**
- Workspace directory path (default: `~/.maestro/workspace`)
- Create directory if not exists

**Step 2: Editor Selection**
- Choose from: Helix, Neovim, Vim, VS Code, Zed, Custom
- Similar to existing Settings menu pattern

**Step 3: AI Provider (Optional)**
- Quick setup: Use environment variable (ANTHROPIC_API_KEY, etc.)
- Or skip for manual configuration later

**Step 4: Theme Selection**
- Preview themes with live switching
- Default: "system" (respects terminal transparency)

### 2.4 Welcome State Machine

```rust
pub enum WelcomeState {
    NotStarted,
    WorkspaceSetup { path: String },
    EditorSelection { selected: usize },
    ProviderSetup { use_env: bool, custom_key: Option<String> },
    ThemeSelection { preview: String },
    Completed,
}

pub struct WelcomeScreen {
    pub state: WelcomeState,
    pub current_step: usize,
    pub total_steps: usize,
    pub workspace_path: String,
    pub selected_editor: String,
    pub skip_provider_setup: bool,
    pub theme_name: String,
}
```

---

## 3. Navigation Redesign: Hub-and-Spoke Model

### 3.1 Current Problem

The current tab bar shows all 10 tabs in a single row:
```
[Dashboard] [Sessions] [Projects] [Analysis] [Conductor] [Memory] [Ktop] [LSPs] [Capabilities] [Settings]
```

As Phase 3-5 adds more capabilities, this becomes untenable.

### 3.2 Proposed Solution: Category Groups with Quick Access

#### Main Categories (Tab Bar)

```
[Hub] [Sessions] [Conductor] [Capabilities] [Settings]
```

**Hub** - Dashboard + Quick Stats (Default landing)
**Sessions** - All session management (tmux integration)
**Conductor** - Track/Task execution (Ralph-style)
**Capabilities** - All Phase 3-5 features (expandable)
**Settings** - Configuration and preferences

#### Expanded Navigation (Alt+Number or Modal)

Pressing `?` or `Alt+0` opens a **Command Palette** overlay:

```
+--------------------------------------------------+
| COMMAND PALETTE                        [Esc: X] |
+--------------------------------------------------+
| > _                                              |
|                                                  |
|  Recent:                                         |
|  • Dashboard (Alt+1)                             |
|  • Sessions (Alt+2)                              |
|  • Conductor (Alt+3)                             |
|                                                  |
|  Capabilities:                                   |
|  • Cron Jobs (c1)                                |
|  • MCP Servers (c2)                              |
|  • Sandbox (c3)                                  |
|  • Channels (c4)                                 |
|  • Web Gateway (c5)                              |
|                                                  |
|  Analysis:                                       |
|  • LeIndex Search (a1)                           |
|  • Code Analysis (a2)                            |
|                                                  |
+--------------------------------------------------+
```

### 3.3 Keybinding Strategy

**Global Keys (Always Active):**
| Key | Action |
|-----|--------|
| `?` | Open Command Palette / Help |
| `q` | Quit (with confirmation if operations running) |
| `Tab` | Cycle forward through main tabs |
| `Shift+Tab` | Cycle backward through main tabs |
| `Alt+1-5` | Jump to main tabs |
| `Esc` | Close modal / Return to previous context |
| `/` | Quick search (context-aware) |
| `:` | Command mode (vim-style) |

**Context Keys (Tab-Specific):**
| Tab | Key | Action |
|-----|-----|--------|
| Hub | `j/k` | Navigate sections |
| Sessions | `n` | New session |
| Sessions | `s` | Switch/attach |
| Sessions | `x` | Kill session |
| Conductor | `s` | Start track |
| Conductor | `p` | Pause |
| Conductor | `r` | Resume |
| Conductor | `Ctrl+r` | Retry task |
| Conductor | `Ctrl+s` | Skip task |
| Capabilities | `1-5` | Switch subsections |

### 3.4 Tab Bar Enhancement

```rust
pub struct TabConfig {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub shortcut: char,
    pub category: TabCategory,
    pub subsections: Vec<Subsection>,
}

pub enum TabCategory {
    Primary,    // Hub, Sessions, Conductor
    Secondary,  // Capabilities
    Utility,    // Settings
}

pub struct Subsection {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,  // e.g., "c1" for Capabilities->Cron
}
```

---

## 4. Pane Layout: Information Density

### 4.1 Hub Tab Layout (Enhanced Dashboard)

```
+------------------------------------------------------------------------------+
| [Hub] [Sessions] [Conductor] [Capabilities] [Settings]         v2.5 | ? help |
+------------------------------------------------------------------------------+
|                          |                                                   |
|  QUICK STATS             |  RECENT SESSIONS                                  |
|  +--------------------+  |  +---------------------------------------------+   |
|  | Projects: 03       |  |  | > maestro-dev [Rust]     ● Running          |   |
|  | Tracks: 05         |  |  |   api-server [Python]    ● Running          |   |
|  | Memories: 127      |  |  |   docs-site [TypeScript] ○ Stopped          |   |
|  | LSPs: 2/3 Active   |  |  +---------------------------------------------+   |
|  +--------------------+  |                                                   |
|                          |  MCP POOL (2/3 Connected)                         |
|  SYSTEM STATUS           |  +---------------------------------------------+   |
|  +--------------------+  |  | ● filesystem    [Ready]                      |   |
|  | Model: claude-3.5  |  |  | ● memory        [Ready]                      |   |
|  | DB: turso (local)  |  |  | ○ github        [Disconnected]               |   |
|  | Features:          |  |  +---------------------------------------------+   |
|  |  ● LeIndex HD      |  |                                                   |
|  |  ● Heartbeat (30m) |  |  QUICK ACTIONS                                    |
|  |  ○ Channels        |  |  +---------------------------------------------+   |
|  +--------------------+  |  | [n] New Session  [/] Search  [c] Conductor    |   |
|                          |  +---------------------------------------------+   |
+------------------------------------------------------------------------------+
| Tab:Switch  ↑↓:Scroll  Alt+1-5:Jump  n:New  s:Switch  /:Search  q:Quit     |
+------------------------------------------------------------------------------+
```

### 4.2 Capabilities Tab Layout (Phase 3-5)

```
+------------------------------------------------------------------------------+
| [Hub] [Sessions] [Conductor] [Capabilities] [Settings]         v2.5 | ? help |
+------------------------------------------------------------------------------+
| [Cron(3)] [MCP(2/3)] [Sandbox] [Channels] [Gateway]                         |
+------------------------------------------------------------------------------+
|                          |                                                   |
|  SCHEDULED JOBS (3)      |  JOB DETAILS                                      |
|  +--------------------+  |  +---------------------------------------------+   |
|  | > heartbeat-engine |  |  | ID: heartbeat-engine                         |   |
|  |   cleanup-memories |  |  | Schedule: */30 * * * * (every 30 min)        |   |
|  |   nightly-backup   |  |  | Type: Agent                                  |   |
|  +--------------------+  |  | Last Run: 2026-02-17 14:30:00                |   |
|                          |  | Next Run: 2026-02-17 15:00:00                |   |
|  [N] New  [E] Edit       |  | Status: ● Enabled                             |   |
|  [D] Delete  [T] Toggle  |  +---------------------------------------------+   |
|                          |                                                   |
+------------------------------------------------------------------------------+
| Tab:Switch  1-5:Section  j/k:Nav  Enter:Select  N:New  E:Edit  T:Toggle    |
+------------------------------------------------------------------------------+
```

### 4.3 Conductor Tab Layout (Enhanced Ralph-Style)

Keep the existing Ralph-style conductor but add:

1. **Header Status Line**: Model, loop mode, current iteration
2. **Focus Indicators**: Clear visual distinction between tree and output panes
3. **Parallel Execution View**: For Phase 4/5 multi-agent support

---

## 5. Capability Presentation: Phase 3-5 Services

### 5.1 Capabilities Tab Structure

The Capabilities tab should have 5 subsections accessible via number keys or left navigation:

**1. Cron Jobs (c1)**
- Table view of scheduled jobs
- Columns: ID, Name, Schedule, Type, Enabled, Last Run
- Actions: New, Edit, Delete, Toggle, Run Now

**2. MCP Servers (c2)**
- List of registered MCP servers with connection status
- Tool count per server
- Actions: Add, Connect, Disconnect, Refresh Tools, View Logs

**3. Sandbox (c3)**
- Security policy display (autonomy level, memory limits, network)
- Available runtimes (Native, WASM, Docker)
- Actions: Change Policy, Enable/Disable Runtimes

**4. Channels (c4)** - *New for Phase 4*
- Telegram, Discord, Slack configuration
- Connection status per channel
- Actions: Configure, Connect, Disconnect, Test

**5. Web Gateway (c5)** - *New for Phase 4*
- Gateway status (running/stopped)
- Connected clients count
- SSE/WebSocket endpoints
- Actions: Start, Stop, View Logs, Pair Device

### 5.2 Capability Status Indicators

```rust
pub enum CapabilityStatus {
    Enabled,        // ● (green)
    Disabled,       // ○ (gray)
    Running,        // ● (green, animated)
    Error,          // x (red)
    Pending,        // ◐ (yellow)
    NotConfigured,  // ○ (muted)
}
```

### 5.3 Quick Actions for Each Capability

```rust
pub struct CapabilityQuickAction {
    pub key: char,
    pub label: &'static str,
    pub action: CapabilityAction,
}

pub enum CapabilityAction {
    Create,
    Edit,
    Delete,
    Toggle,
    Connect,
    Disconnect,
    Refresh,
    ViewLogs,
    Test,
}
```

---

## 6. Keybinding Strategy: Scalable Design

### 6.1 Global Keybindings (Always Active)

```
+------------------+----------------------------------------+
| Key              | Action                                 |
+------------------+----------------------------------------+
| ?                | Help / Command Palette                 |
| q                | Quit                                   |
| Tab              | Next tab                               |
| Shift+Tab        | Previous tab                           |
| Alt+1            | Hub (Dashboard)                        |
| Alt+2            | Sessions                               |
| Alt+3            | Conductor                              |
| Alt+4            | Capabilities                           |
| Alt+5            | Settings                               |
| /                | Quick search (context-aware)           |
| :                | Command mode                           |
| Esc              | Close modal / Cancel                   |
| Ctrl+l           | Redraw screen                          |
| Shift+T          | Cycle theme                            |
+------------------+----------------------------------------+
```

### 6.2 Command Mode (vim-style `:`)

Pressing `:` opens a command input at the bottom:

```
: cron enable heartbeat-engine
: mcp connect filesystem
: sandbox set-policy autonomous
: channel start telegram
: gateway start
```

### 6.3 Quick Search (`/`)

Context-aware search that changes behavior based on current tab:

- **Hub**: Search sessions, projects, memories
- **Sessions**: Filter session list
- **Conductor**: Search tracks and tasks
- **Capabilities**: Search across all capability items

---

## 7. Implementation Phases

### Phase 1: Welcome Screen (2-3 days)

**Files to Create/Modify:**
- `crates/cockpit/src/welcome/mod.rs` - Welcome module
- `crates/cockpit/src/welcome/screen.rs` - Welcome screen rendering
- `crates/cockpit/src/welcome/wizard.rs` - Onboarding wizard logic
- `crates/cockpit/src/app.rs` - Add welcome state detection

**Tasks:**
1. Create `WelcomeScreen` struct and state machine
2. Implement step-by-step wizard rendering
3. Add first-time detection logic
4. Create marker file on completion
5. Add welcome state to main app loop

### Phase 2: Navigation Redesign (3-4 days)

**Files to Modify:**
- `crates/cockpit/src/app.rs` - Tab structure changes
- `crates/cockpit/src/command_palette/mod.rs` - New: Command palette
- `crates/cockpit/src/command_palette/render.rs` - New: Palette rendering

**Tasks:**
1. Consolidate tabs to 5 main categories
2. Implement command palette overlay
3. Add Alt+number shortcuts
4. Implement context-aware search
5. Add command mode (`:`)

### Phase 3: Capabilities Tab Expansion (2-3 days)

**Files to Modify:**
- `crates/cockpit/src/tabs/capabilities.rs` - Extend for 5 subsections
- `crates/cockpit/src/tabs/channels.rs` - New: Channels subsection
- `crates/cockpit/src/tabs/gateway.rs` - New: Web Gateway subsection

**Tasks:**
1. Extend CapabilitiesSection enum for 5 subsections
2. Implement Channels configuration UI
3. Implement Web Gateway status UI
4. Add quick actions for each subsection
5. Wire up to maestro-core services

### Phase 4: Enhanced Hub Dashboard (2 days)

**Files to Modify:**
- `crates/cockpit/src/tabs/dashboard.rs` - Enhanced layout
- `crates/cockpit/src/tabs/quick_actions.rs` - New: Quick actions panel

**Tasks:**
1. Redesign dashboard layout (4-quadrant)
2. Add quick actions panel
3. Enhance system status display
4. Add MCP pool status section
5. Implement keyboard shortcuts for quick actions

### Phase 5: Polish and Testing (2 days)

**Tasks:**
1. Update footer to reflect new keybindings
2. Add transition animations between tabs
3. Implement help modal for each tab
4. Write integration tests
5. Update documentation

---

## 8. Component Architecture

### 8.1 New Module Structure

```
crates/cockpit/src/
├── app.rs                    # Main app (modified)
├── welcome/                  # NEW
│   ├── mod.rs
│   ├── screen.rs             # Welcome screen rendering
│   └── wizard.rs             # Onboarding wizard
├── command_palette/          # NEW
│   ├── mod.rs
│   ├── render.rs             # Palette overlay
│   └── search.rs             # Fuzzy search logic
├── tabs/
│   ├── mod.rs
│   ├── dashboard.rs          # Enhanced hub
│   ├── sessions.rs
│   ├── conductor_bridge.rs   # NEW: Conductor tab bridge
│   ├── capabilities/
│   │   ├── mod.rs
│   │   ├── cron.rs
│   │   ├── mcp.rs
│   │   ├── sandbox.rs
│   │   ├── channels.rs       # NEW
│   │   └── gateway.rs        # NEW
│   └── settings.rs
├── conductor/                # Existing (minimal changes)
└── theme.rs
```

### 8.2 State Management

```rust
pub struct CockpitState {
    pub welcome: Option<WelcomeScreen>,
    pub current_tab: TabId,
    pub command_palette: Option<CommandPaletteState>,
    pub search_query: Option<String>,
    pub toasts: ToastQueue,
}

pub enum TabId {
    Hub = 0,
    Sessions = 1,
    Conductor = 2,
    Capabilities = 3,
    Settings = 4,
}
```

---

## 9. Visual Design Specifications

### 9.1 Color Usage

| Element | Color | Purpose |
|---------|-------|---------|
| Active tab | Cyan (accent) | Current context |
| Inactive tab | Muted gray | Available options |
| Running status | Green | Active/healthy |
| Warning | Yellow | Attention needed |
| Error | Red | Problem state |
| Disabled | Dark gray | Unavailable |
| Selected item | Highlight background | Focus indicator |

### 9.2 Border Styles

| Element | Border Type |
|---------|-------------|
| Active tab content | Double |
| Inactive content | Rounded |
| Modals | Thick |
| Sections | Rounded |
| Selected item | Double left border |

### 9.3 Typography

| Element | Style |
|---------|-------|
| Tab titles | Bold |
| Section headers | Bold + accent color |
| Status text | Normal |
| Help text | Italic + muted |
| Numbers/stats | Bold |

---

## 10. Backward Compatibility

### 10.1 Configuration Migration

- Existing `~/.maestro/config.toml` continues to work
- Welcome screen only shows for new users
- All existing keybindings remain functional
- Alt+6-9 keys redirect to Capabilities subsections

### 10.2 Feature Flags

```toml
[cockpit]
welcome_enabled = true
command_palette_enabled = true
compact_tabs = false  # Use 5-tab layout vs 10-tab layout
```

---

## 11. Testing Strategy

### 11.1 Unit Tests

- Welcome state machine transitions
- Command palette search ranking
- Tab navigation cycling
- Keybinding routing

### 11.2 Integration Tests

- First-time user flow
- Tab switching with running operations
- Modal overlay behavior
- Capability subsection navigation

### 11.3 Visual Tests

- Theme switching
- Border style transitions
- Toast notification positioning
- Modal centering

---

## 12. Success Metrics

1. **Onboarding Completion Rate** - Users completing welcome wizard
2. **Tab Navigation Speed** - Time to switch between contexts
3. **Feature Discovery** - Users accessing Phase 3-5 capabilities
4. **Error Rate** - Keybinding conflicts or navigation errors
5. **User Satisfaction** - Feedback on TUI experience

---

## 13. Critical Files for Implementation

| File | Purpose |
|------|---------|
| `crates/cockpit/src/app.rs` | Main application state machine and UI routing (5000+ lines, central hub) |
| `crates/cockpit/src/tabs/capabilities.rs` | Existing capabilities tab to extend with Channels/Gateway |
| `crates/cockpit/src/conductor/keybindings.rs` | Keybinding patterns to follow for new features |
| `crates/cockpit/src/tabs/dashboard.rs` | Dashboard patterns to enhance for Hub redesign |
| `crates/cockpit/src/theme.rs` | Theme system to maintain consistency |
