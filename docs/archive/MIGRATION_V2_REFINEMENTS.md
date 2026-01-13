# Maestro v2 Refinements Migration Guide

This document provides comprehensive migration guidance for upgrading to Maestro v2 Refinements, covering all sub-tracks and their associated changes.

## Table of Contents

- [Overview of Sub-Tracks](#overview-of-sub-tracks)
  - [Sub-Track 01: Installer Refinements](#sub-track-01-installer-refinements)
  - [Sub-Track 02: Cross-Platform Fixes](#sub-track-02-cross-platform-fixes)
  - [Sub-Track 03: DuckDB+SQLite Migration](#sub-track-03-duckdbsqlite-migration)
  - [Sub-Track 04: LeIndex Integration](#sub-track-04-leindex-integration)
  - [Sub-Track 05: CCv3 Features](#sub-track-05-ccv3-features)
- [Upgrade Path for Existing Users](#upgrade-path-for-existing-users)
- [Rollback Procedures](#rollback-procedures)
- [Breaking Changes and Compatibility Notes](#breaking-changes-and-compatibility-notes)
- [New Features Summary](#new-features-summary)

## Overview of Sub-Tracks

### Sub-Track 01: Installer Refinements

**Components:**
- MCP (Maestro Control Panel) setup
- Configuration backup system
- TypeScript hooks integration

**Key Changes:**
- Streamlined installation process with automated dependency resolution
- Configuration backup before upgrades with versioned snapshots
- TypeScript hooks for pre/post installation validation
- Improved error handling and rollback capabilities

**Migration Impact:**
- Existing installations will be automatically migrated during upgrade
- Configuration files will be backed up to `~/.maestro/backups/` with timestamps
- TypeScript hooks require Node.js 18+ (automatically installed if missing)

### Sub-Track 02: Cross-Platform Fixes

**Components:**
- Python environment detection
- Daemon lifecycle management
- Rich markup rendering

**Key Changes:**
- Enhanced Python version detection across platforms (Windows, macOS, Linux)
- Robust daemon process management with automatic recovery
- Cross-platform Rich markup rendering with fallback mechanisms
- Improved terminal compatibility and color scheme detection

**Migration Impact:**
- Python detection is now more reliable, especially on Windows systems
- Daemon processes will automatically restart after crashes or system reboots
- Rich markup rendering will gracefully degrade on unsupported terminals

### Sub-Track 03: DuckDB+SQLite Migration

**Components:**
- Zero-infrastructure storage backend
- DuckDB integration
- SQLite compatibility layer

**Key Changes:**
- Migration from traditional database systems to embedded DuckDB
- Automatic schema migration during upgrade
- SQLite compatibility layer for backward compatibility
- Improved query performance and reduced resource usage

**Migration Impact:**
- Existing data will be automatically migrated to DuckDB format
- Database files will be converted during first launch (may take several minutes for large datasets)
- SQLite-based tools can still access data through compatibility layer
- Reduced disk space usage (typically 20-30% reduction)

### Sub-Track 04: LeIndex Integration

**Components:**
- 5-layer code analysis engine
- Tantivy search backend
- MCP server integration

**Key Changes:**
- Advanced code analysis with AST, CFG, DFG, and call graph layers
- Tantivy-based search with sub-second response times
- MCP server for distributed analysis capabilities
- Enhanced indexing performance and accuracy

**Migration Impact:**
- Existing indexes will be rebuilt using new 5-layer analysis
- Search performance will improve significantly (5-10x faster)
- MCP server requires additional 500MB RAM for optimal performance
- New analysis capabilities available through updated APIs

### Sub-Track 05: CCv3 Features

**Components:**
- Dashboard UI
- Update Wizard
- Daemon management
- Handoff system
- Fallback mechanisms

**Key Changes:**
- New web-based dashboard with real-time monitoring
- Interactive update wizard for guided upgrades
- Enhanced daemon management with health monitoring
- Improved handoff system for agent coordination
- Comprehensive fallback mechanisms for error recovery

**Migration Impact:**
- Dashboard accessible at `http://localhost:8765` after upgrade
- Update wizard provides step-by-step guidance for major version upgrades
- Daemon management includes automatic health checks and alerts
- Handoff system improves multi-agent workflow reliability

## Upgrade Path for Existing Users

### Prerequisites

1. **Backup your data:**
   ```bash
   maestro backup create --full
   ```

2. **Check system requirements:**
   - Python 3.11+
   - Node.js 18+ (for TypeScript hooks)
   - 4GB RAM minimum (8GB recommended)
   - 2GB free disk space

3. **Review current configuration:**
   ```bash
   maestro config show
   ```

### Upgrade Procedure

1. **Stop all Maestro services:**
   ```bash
   maestro daemon stop --all
   ```

2. **Run the upgrade command:**
   ```bash
   maestro upgrade --refinements
   ```

3. **Follow the interactive prompts:**
   - Review component changes
   - Confirm data migration
   - Accept new configuration options

4. **Verify the upgrade:**
   ```bash
   maestro version
   maestro health check
   ```

5. **Restart services:**
   ```bash
   maestro daemon start --all
   ```

### Post-Upgrade Steps

1. **Verify data integrity:**
   ```bash
   maestro database verify
   ```

2. **Test new features:**
   ```bash
   maestro dashboard start
   # Access at http://localhost:8765
   ```

3. **Review new configuration options:**
   ```bash
   maestro config edit
   ```

## Rollback Procedures

### Automatic Rollback

If the upgrade fails, Maestro will automatically:
1. Restore configuration from backup
2. Revert database changes
3. Restart previous version services

### Manual Rollback

1. **Stop all services:**
   ```bash
   maestro daemon stop --all
   ```

2. **Restore from backup:**
   ```bash
   maestro backup restore --latest
   ```

3. **Reinstall previous version:**
   ```bash
   maestro install --version=2.0.0
   ```

4. **Verify rollback:**
   ```bash
   maestro version
   maestro health check
   ```

## Breaking Changes and Compatibility Notes

### Configuration Changes

- **Deprecated options:**
  - `legacy_search_enabled` → Replaced by `search.engine`
  - `database.host` → Replaced by embedded DuckDB
  - `agent.coordination_mode` → Replaced by `workflow.orchestration`

- **New required options:**
  - `leindex.layers` (default: 5)
  - `dashboard.enabled` (default: true)
  - `fallbacks.strategy` (default: "graceful")

### API Changes

- **Search API:**
  - `POST /api/search` now returns enhanced results with analysis layers
  - Query format changed to support multi-layer analysis

- **Agent API:**
  - Handoff endpoints renamed from `/api/agent/handoff` to `/api/workflow/handoff`
  - New fallback mechanisms require updated error handling

### Database Schema

- **Migrated tables:**
  - `code_analysis` → `leindex_analysis`
  - `search_index` → `tantivy_index`
  - `agent_sessions` → `workflow_sessions`

- **New tables:**
  - `leindex_layers` (AST, CFG, DFG, CallGraph, Semantic)
  - `dashboard_metrics`
  - `fallback_events`

### Compatibility Matrix

| Component | v1.x | v2.0 | v2 Refinements |
|-----------|------|------|----------------|
| Search API | ✓ | ✓ | ✓ (enhanced) |
| Agent API | ✓ | ✓ | ✓ (renamed endpoints) |
| Database | SQLite | PostgreSQL | DuckDB |
| Dashboard | Basic | Improved | Full-featured |
| Indexing | Basic | Advanced | 5-layer |

## New Features Summary

### Enhanced Analysis Capabilities

- **5-Layer Code Analysis:** AST, CFG, DFG, Call Graph, and Semantic analysis
- **Tantivy Search:** Sub-second search responses with advanced filtering
- **Cross-Reference Analysis:** Improved code understanding and navigation

### Improved Reliability

- **Automatic Fallbacks:** Graceful degradation during errors
- **Health Monitoring:** Real-time daemon and service monitoring
- **Automatic Recovery:** Self-healing capabilities for common issues

### Developer Experience

- **Interactive Dashboard:** Real-time monitoring and control
- **Update Wizard:** Guided upgrade process
- **Enhanced CLI:** Improved command structure and help system

### Performance Improvements

- **Faster Indexing:** 5-10x improvement in search performance
- **Reduced Memory:** 20-30% lower memory footprint
- **Embedded Database:** Zero-infrastructure storage with DuckDB

### Migration Checklist

- [ ] Review breaking changes and compatibility notes
- [ ] Backup all data and configuration
- [ ] Verify system requirements
- [ ] Run upgrade procedure
- [ ] Test new features and functionality
- [ ] Update any custom integrations or scripts
- [ ] Train team on new dashboard and features

## Support and Resources

For migration assistance:
- **Documentation:** https://maestro-docs.example.com/migration
- **Community Forum:** https://community.maestro.example.com
- **Support Ticket:** https://support.maestro.example.com

## Troubleshooting

### Common Issues

**Issue: Upgrade hangs during database migration**
- Solution: Increase system memory or run with `--batch-size=1000`

**Issue: Dashboard fails to start**
- Solution: Check Node.js version and run `maestro dashboard setup`

**Issue: Search returns incomplete results**
- Solution: Rebuild index with `maestro leindex rebuild --full`

### Logs and Diagnostics

```bash
# View upgrade logs
maestro logs upgrade

# Run diagnostics
maestro diagnostics run

# Check system health
maestro health check --detailed
```

## Conclusion

The Maestro v2 Refinements upgrade provides significant improvements in performance, reliability, and functionality while maintaining backward compatibility for most use cases. By following this migration guide and the recommended upgrade path, you can smoothly transition to the new version with minimal disruption to your workflows.

For complex environments or custom integrations, we recommend testing the upgrade in a staging environment before applying to production systems.