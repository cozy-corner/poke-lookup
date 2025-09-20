#!/bin/bash

# Git hooks setup script for poke-lookup project

set -e

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "Setting up Git hooks for poke-lookup..."

# Ensure .git/hooks directory exists
if [ ! -d "$HOOKS_DIR" ]; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Install pre-commit hook
if [ -f "$SCRIPT_DIR/hooks/pre-commit" ]; then
    cp "$SCRIPT_DIR/hooks/pre-commit" "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "✅ Pre-commit hook installed"
else
    echo "❌ Pre-commit hook not found in scripts/hooks/"
    exit 1
fi

echo ""
echo "Git hooks setup complete!"
echo ""
echo "The pre-commit hook will:"
echo "  • Run cargo fmt --check"
echo "  • Run cargo clippy"
echo "  • Run cargo test"
echo ""
echo "To bypass the hook in emergencies, use: git commit --no-verify"