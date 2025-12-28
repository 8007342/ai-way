#!/bin/bash
# ============================================================================
# install-hooks.sh - Install git hooks for development
#
# Run this once after cloning to set up automatic checksum updates.
#
# What it does:
# - Installs pre-commit hook that auto-updates .integrity/checksums.sha256
# - Ensures checksums are always in sync with code
#
# Usage:
#   ./scripts/install-hooks.sh
# ============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks..."

# Create pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'HOOK'
#!/bin/bash
# ============================================================================
# pre-commit hook: Auto-update integrity checksums
#
# When any .sh file is modified, automatically regenerate the checksums
# manifest and include it in the commit.
# ============================================================================

if git diff --cached --name-only | grep -q '\.sh$'; then
    echo "Updating integrity checksums..."

    REPO_ROOT="$(git rev-parse --show-toplevel)"
    mkdir -p "$REPO_ROOT/.integrity"

    (
        cd "$REPO_ROOT"
        find . -name "*.sh" -type f -not -path "./.git/*" | sort | xargs sha256sum | sed 's|  \./|  |g'
    ) > "$REPO_ROOT/.integrity/checksums.sha256"

    git add "$REPO_ROOT/.integrity/checksums.sha256"
    echo "Checksums updated and staged."
fi

exit 0
HOOK

chmod +x "$HOOKS_DIR/pre-commit"

echo "Done! Git hooks installed:"
echo "  - pre-commit: Auto-updates integrity checksums"
