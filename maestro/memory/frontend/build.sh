#!/bin/bash

# Maestro Memory Dashboard - Build Script
# This script builds the React frontend for the Maestro Memory Dashboard

set -e

# Use the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$SCRIPT_DIR"
DIST_DIR="$FRONTEND_DIR/dist"

echo "================================"
echo "Maestro Memory Dashboard Builder"
echo "================================"
echo ""

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "Error: Node.js is not installed"
    echo "Please install Node.js 18+ from https://nodejs.org/"
    exit 1
fi

# Check Node.js version
NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    echo "Error: Node.js 18+ is required (current version: $(node -v))"
    exit 1
fi

echo "✓ Node.js version: $(node -v)"
echo ""

# Change to frontend directory
cd "$FRONTEND_DIR"

# Check if package.json exists
if [ ! -f "package.json" ]; then
    echo "Error: package.json not found in $FRONTEND_DIR"
    exit 1
fi

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
    echo ""
fi

# Clean previous build
if [ -d "$DIST_DIR" ]; then
    echo "Cleaning previous build..."
    rm -rf "$DIST_DIR"
    echo ""
fi

# Build the project
echo "Building frontend..."
npm run build

# Check if build succeeded
if [ -d "$DIST_DIR" ]; then
    echo ""
    echo "================================"
    echo "✓ Build completed successfully!"
    echo "================================"
    echo ""
    echo "Built files location: $DIST_DIR"
    echo "Total size: $(du -sh "$DIST_DIR" | cut -f1)"
    echo ""
    echo "To serve the dashboard:"
    echo "  maestro memory serve"
    echo ""
    echo "Or directly with uvicorn:"
    echo "  cd $SCRIPT_DIR/../.."
    echo "  python -m maestro.memory.cli serve"
    echo ""
else
    echo ""
    echo "================================"
    echo "✗ Build failed!"
    echo "================================"
    echo ""
    echo "Please check the error messages above."
    exit 1
fi
