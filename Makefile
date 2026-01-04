# Maestro Makefile
.PHONY: help build install clean test tui-build tui-install tui-clean tui-test install-all

# Default target
help:
	@echo "Maestro 2.0 - Unified Development Framework"
	@echo ""
	@echo "Python targets:"
	@echo "  make install       Install Maestro Python CLI"
	@echo "  make test          Run Python tests"
	@echo "  make clean         Clean Python build artifacts"
	@echo ""
	@echo "TUI targets (Go):"
	@echo "  make tui-build     Build maestro-tui Go binary"
	@echo "  make tui-install   Install maestro-tui to ~/.local/bin"
	@echo "  make tui-test      Run Go tests"
	@echo "  make tui-clean     Clean Go build artifacts"
	@echo ""
	@echo "Combined targets:"
	@echo "  make install-all   Install both Python and TUI"
	@echo "  make test-all      Run all tests"
	@echo "  make clean         Clean all build artifacts"

# Python variables
PYTHON=python3
PIP=pip

# TUI variables (Go)
TUI_DIR=maestro/tui
TUI_BINARY=maestro-tui
TUI_BUILD_DIR=$(TUI_DIR)/build
TUI_INSTALL_DIR=$(HOME)/.local/bin

# ============================================================================
# Python targets
# ============================================================================

install:
	$(PIP) install -e .

test:
	$(PYTHON) -m pytest maestro/memory/tests/ -v

clean-py:
	rm -rf dist/ build/ *.egg-info
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete

# ============================================================================
# TUI targets (Go)
# ============================================================================

tui-build:
	cd $(TUI_DIR) && go build -o $(TUI_BUILD_DIR)/$(TUI_BINARY) ./cmd/maestro-tui
	@echo "✅ TUI binary built: $(TUI_BUILD_DIR)/$(TUI_BINARY)"

tui-install: tui-build
	mkdir -p $(TUI_INSTALL_DIR)
	cp $(TUI_BUILD_DIR)/$(TUI_BINARY) $(TUI_INSTALL_DIR)/
	@echo "✅ Maestro TUI installed to $(TUI_INSTALL_DIR)/$(TUI_BINARY)"

tui-clean:
	cd $(TUI_DIR) && go clean
	rm -rf $(TUI_BUILD_DIR)
	@echo "✅ TUI build artifacts cleaned"

tui-test:
	cd $(TUI_DIR) && go test ./...

# ============================================================================
# Combined targets
# ============================================================================

install-all: install tui-install
	@echo "✅ Maestro + TUI installed successfully"

test-all: test tui-test
	@echo "✅ All tests passed"

clean: clean-py tui-clean
	@echo "✅ All build artifacts cleaned"

# ============================================================================
# Development targets
# ============================================================================

dev-install:
	$(PIP) install -e ".[dev]"
	cd $(TUI_DIR) && go mod tidy

dev-tui:
	cd $(TUI_DIR) && go run ./cmd/maestro-tui
