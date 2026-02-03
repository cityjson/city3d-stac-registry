#!/bin/sh
#
# Setup script for git hooks
# Run this once after cloning the repository
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$ROOT_DIR/.githooks"

echo "🔧 Setting up git hooks for cityjson-stac..."

# Configure git to use our hooks directory
git config core.hooksPath .githooks

# Make hooks executable
chmod +x "$HOOKS_DIR/pre-commit"

echo "✅ Git hooks configured!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Runs format, lint, and test checks"
echo ""
echo "To bypass hooks (emergency only): git commit --no-verify"
echo "To disable hooks: git config --unset core.hooksPath"
