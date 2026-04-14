#!/bin/sh
# Version bump script — single source of truth for all version references
# in bote. Mirrors the shape of cyrius's scripts/version-bump.sh.
#
# Usage: ./scripts/version-bump.sh 1.5.2

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Current: $(cat VERSION)"
    exit 1
fi

NEW="$1"
OLD=$(cat VERSION | tr -d '[:space:]')

if [ "$NEW" = "$OLD" ]; then
    echo "Already at $OLD"
    exit 0
fi

# 1. VERSION file (source of truth — CI checks this against cyrius.toml)
echo "$NEW" > VERSION

# 2. cyrius.toml [package] version (what `cyrius build` reads)
sed -i "s/^version = \"$OLD\"/version = \"$NEW\"/" cyrius.toml

# 3. src/dispatch.cyr — _bote_server_version() is what the MCP initialize
#    handshake reports to clients. Must match VERSION or `initialize` lies.
if [ -f src/dispatch.cyr ]; then
    if grep -q "_bote_server_version" src/dispatch.cyr; then
        sed -i "s|fn _bote_server_version() { return \"$OLD\"; }|fn _bote_server_version() { return \"$NEW\"; }|" src/dispatch.cyr
    else
        echo "  warning: _bote_server_version not found in src/dispatch.cyr" >&2
    fi
fi

# 4. CHANGELOG.md — add dated section if the new version isn't already present
if ! grep -q "## \[$NEW\]" CHANGELOG.md 2>/dev/null; then
    # Insert right after the top-level intro line so the newest version is
    # always first under the heading. Keep-a-Changelog style.
    sed -i "/^All notable changes to bote are documented here\.$/a\\
\\
## [$NEW] — $(date +%Y-%m-%d) — TODO\\
\\
TODO: write release notes." CHANGELOG.md 2>/dev/null || true
    CHANGELOG_ADDED=1
else
    CHANGELOG_ADDED=0
fi

# 4. Cross-check: cyrius.toml and VERSION must agree (this is what CI verifies)
CTOML=$(grep '^version = ' cyrius.toml | sed 's/^version = "\(.*\)"/\1/')
VFILE=$(cat VERSION | tr -d '[:space:]')
if [ "$CTOML" != "$VFILE" ]; then
    echo "ERROR: cyrius.toml=$CTOML disagrees with VERSION=$VFILE" >&2
    exit 1
fi

echo "$OLD -> $NEW"
echo ""
echo "Updated:"
echo "  VERSION"
echo "  cyrius.toml"
echo "  src/dispatch.cyr (_bote_server_version)"
if [ "$CHANGELOG_ADDED" = "1" ]; then
    echo "  CHANGELOG.md (placeholder section inserted — fill it in)"
else
    echo "  CHANGELOG.md (section for $NEW already present)"
fi
echo ""
echo "Still manual (when applicable):"
echo "  - Flesh out CHANGELOG.md Added/Changed/Fixed/Security sections"
echo "  - docs/development/roadmap.md status line"
echo "  - 'cyrius = \"X.Y.Z\"' pin in cyrius.toml if compiler version changed"
echo "  - .cyrius-toolchain pin if compiler version changed (CI installs this)"
echo "  - Run: cyrius test tests/bote.tcyr && cyrius bench tests/bote.bcyr"
echo "  - Tag + push: git tag -a $NEW -m \"bote $NEW\" && git push --tags"
