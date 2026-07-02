#!/bin/sh
# Version bump script — single source of truth for all version references
# in bote. Mirrors the shape of cyrius's scripts/version-bump.sh.
#
# Usage: ./scripts/version-bump.sh 2.7.9
#
# Touches, in order:
#   1. VERSION                       the single source of truth. cyrius.cyml's
#                                    [package].version is "${file:VERSION}", so
#                                    the manifest tracks this automatically —
#                                    there is deliberately NO manifest edit.
#   2. src/dispatch.cyr              _bote_server_version() literal — the string
#                                    the MCP `initialize` handshake reports.
#   3. CHANGELOG.md                  renames `## [Unreleased]` to the dated
#                                    version header and seeds a fresh empty
#                                    `## [Unreleased]` on top (bote's 2.7.0
#                                    accumulator convention).
#   4. dist/bote.cyr, dist/bote-core.cyr (+ .deps)
#                                    regenerated via `cyrius distlib` — their
#                                    headers embed VERSION, so this MUST run
#                                    after step 1.

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Current: $(cat VERSION)"
    exit 1
fi

NEW="$1"
OLD=$(cat VERSION | tr -d '[:space:]')
DATE=$(date +%Y-%m-%d)

if [ "$NEW" = "$OLD" ]; then
    echo "Already at $OLD — nothing to bump."
    echo "(To regenerate dist without bumping: cyrius distlib && cyrius distlib core)"
    exit 0
fi

# Sanity: the manifest must interpolate VERSION, or a bump here would silently
# drift from what `cyrius build` reports. bote moved cyrius.toml -> cyrius.cyml;
# the version field is "${file:VERSION}" (no literal to edit).
if ! grep -q 'version = "${file:VERSION}"' cyrius.cyml 2>/dev/null; then
    echo "ERROR: cyrius.cyml [package].version is not \"\${file:VERSION}\" —" >&2
    echo "       the manifest no longer tracks VERSION; fix that before bumping." >&2
    exit 1
fi

# 1. VERSION file (source of truth; cyrius.cyml interpolates it).
echo "$NEW" > VERSION

# 2. src/dispatch.cyr — _bote_server_version() is what the MCP `initialize`
#    handshake reports to clients. Must match VERSION or `initialize` lies.
if [ -f src/dispatch.cyr ]; then
    if grep -q "_bote_server_version" src/dispatch.cyr; then
        sed -i "s|fn _bote_server_version() { return \"$OLD\"; }|fn _bote_server_version() { return \"$NEW\"; }|" src/dispatch.cyr
        # Confirm the literal actually moved — guards against an OLD with a
        # -hotfix suffix or reformatting the sed pattern silently missed.
        if grep -q "_bote_server_version() { return \"$OLD\"; }" src/dispatch.cyr; then
            echo "ERROR: failed to update _bote_server_version (still \"$OLD\") —" >&2
            echo "       update src/dispatch.cyr by hand." >&2
            exit 1
        fi
    else
        echo "  warning: _bote_server_version not found in src/dispatch.cyr" >&2
    fi
fi

# 3. CHANGELOG.md — bote's 2.7.0 accumulator convention: the `## [Unreleased]`
#    section accrues entries during the cycle; at release we RENAME it to the
#    dated version header and seed a fresh empty `## [Unreleased]` on top.
#    Anchored on the EXACT line and fired ONCE (cyrius v5.8.49 lesson: a loose
#    substring match inserts spurious headers into narrative CHANGELOG text).
if ! grep -q "^## \[$NEW\]" CHANGELOG.md 2>/dev/null; then
    if grep -q "^## \[Unreleased\]$" CHANGELOG.md; then
        awk -v new="$NEW" -v date="$DATE" '
            !seeded && $0 == "## [Unreleased]" {
                print "## [Unreleased]"
                print ""
                print "_(empty)_"
                print ""
                print "## [" new "] — " date " — TODO: headline"
                seeded = 1
                next
            }
            { print }
        ' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
        CHANGELOG_STATE=seeded
    else
        echo "  warning: no '## [Unreleased]' header in CHANGELOG.md — add the" >&2
        echo "           '## [$NEW] — $DATE' section by hand." >&2
        CHANGELOG_STATE=manual
    fi
else
    CHANGELOG_STATE=present
fi

# 4. Regenerate the distributable bundles (headers embed VERSION → after step 1):
#      cyrius distlib       -> dist/bote.cyr      (+ dist/bote.deps)
#      cyrius distlib core  -> dist/bote-core.cyr (+ dist/bote-core.deps)
if command -v cyrius >/dev/null 2>&1; then
    if cyrius distlib >/dev/null 2>&1 && cyrius distlib core >/dev/null 2>&1; then
        DIST_STATE=regen
    else
        DIST_STATE=failed
        echo "  ! cyrius distlib failed — regenerate dist/ by hand" >&2
    fi
else
    DIST_STATE=skipped
fi

echo "$OLD -> $NEW"
echo ""
echo "cyrius pin:"
grep '^cyrius = ' cyrius.cyml | sed 's/^/  /'
echo ""
echo "Updated:"
echo "  VERSION"
echo "  src/dispatch.cyr (_bote_server_version)"
case "$CHANGELOG_STATE" in
    seeded)  echo "  CHANGELOG.md (## [$NEW] section seeded — fill in the headline + notes)" ;;
    present) echo "  CHANGELOG.md (## [$NEW] section already present — left as-is)" ;;
    manual)  echo "  CHANGELOG.md (NOT edited — see warning above)" ;;
esac
case "$DIST_STATE" in
    regen)   echo "  dist/bote.cyr, dist/bote-core.cyr (+ .deps) — regenerated" ;;
    failed)  echo "  dist/ — regen FAILED; run: cyrius distlib && cyrius distlib core" ;;
    skipped) echo "  dist/ — NOT regenerated (cyrius not on PATH); run: cyrius distlib && cyrius distlib core" ;;
esac
echo ""
echo "Still manual:"
echo "  - Flesh out the CHANGELOG.md ## [$NEW] headline + Added/Changed/Fixed/Security"
echo "  - 'cyrius = \"X.Y.Z\"' pin in cyrius.cyml if the toolchain moved"
echo "  - Run the suites: for t in tests/*.tcyr; do cyrius test \"\$t\"; done"
echo "  - Tag + push: git tag -a $NEW -m \"bote $NEW\" && git push --tags"
