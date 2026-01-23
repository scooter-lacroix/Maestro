# Maestro Makefile
.PHONY: help build install clean test install-all

# Default target
help:
	@echo "Maestro 2.5 - Rust-First Unified Development Framework"
	@echo ""
	@echo "Rust targets:"
	@echo "  make build         Build Rust workspace (release mode)"
	@echo "  make install        Install Rust binaries to ~/.cargo/bin"
	@echo "  make test          Run Rust tests"
	@echo "  make clean         Clean Rust build artifacts"
	@echo ""
	@echo "Combined targets:"
	@echo "  make install-all   Build and install all Maestro components"

# Rust variables
CARGO=cargo
WORKSPACE_ROOT=.
INSTALL_DIR=$(HOME)/.cargo/bin

# ============================================================================
# Rust targets
# ============================================================================

build:
	$(CARGO) build --workspace --release
	@echo "✅ Rust binaries built: $(INSTALL_DIR)/maestro, $(INSTALL_DIR)/maestro-setup"

install: build
	@echo "✅ Rust binaries installed via cargo to $(INSTALL_DIR)/"

test:
	$(CARGO) test --workspace

clean:
	$(CARGO) clean
	@echo "✅ Rust build artifacts cleaned"

# Development targets
dev-build:
	$(CARGO) build --workspace

dev-test:
	$(CARGO) test --workspace

# Check code (faster than full build)
check:
	$(CARGO) check --workspace

# Policy checks - enforce architectural rules
policy-check:
	@echo "Checking for forbidden maestro.tldr imports outside archive..."
	@rg -n "maestro\.tldr" --glob '!maestro/archive/**' --glob '!*.txt' --glob '!**/tracks.md' --glob '!**/plan.md' --glob '!Makefile' --glob '!**/SKILL.md' --glob '!**/spec.md' . && echo "❌ ERROR: Found maestro.tldr references outside archive/" && exit 1 || echo "✅ No maestro.tldr imports outside archive/"
	@echo "Checking for archive/tldr execution paths in runtime code..."
	@rg -n "from.*archive.*tldr|import.*archive.*tldr" --glob '!*.txt' --glob '!*.md' --glob '!maestro/archive/**' maestro/ && echo "❌ ERROR: Found archive/tldr imports in runtime code" && exit 1 || echo "✅ No archive/tldr execution paths"

# Run clippy for linting
lint:
	$(CARGO) clippy --workspace --all-targets

# Format code
fmt:
	$(CARGO) fmt --all

# Install from local source (force reinstall)
install-local:
	$(CARGO) install --path crates/cli --force
	$(CARGO) install --path maestro/leindex/rust --bin maestro-setup --force
	@echo "✅ Maestro binaries installed to $(INSTALL_DIR)/"
