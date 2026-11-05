#!/bin/bash
# bote's regular-expression engine against JavaScript's RegExp under the u flag, the way the
# official MCP client's Ajv compiles a JSON Schema `pattern`: 60,000 random patterns and ten
# strings each, every verdict compared, and whether each pattern compiles at all.
#
# Not a CI step: it needs node (no npm). Run it by hand at a release that touches
# src/regex.cyr. It writes only under build/regex-diff. CI builds the comparison program,
# so it does not rot.
#
#   ./scripts/regex-diff.sh
set -u
cd "$(dirname "$0")/.." || exit 1
command -v node >/dev/null 2>&1 || { echo "regex-diff: needs node" >&2; exit 1; }
mkdir -p build/regex-diff
cyrius build tests/fixtures/regex_diff.cyr build/regex-diff/compare >/dev/null 2>&1 \
  || { echo "regex-diff: the comparison program does not build" >&2; exit 1; }
{ node tests/fixtures/regex_diff_gen.mjs 11 20000
  VALID=1 node tests/fixtures/regex_diff_gen.mjs 12 20000
  VALID=1 node tests/fixtures/regex_diff_gen.mjs 13 20000; } > build/regex-diff/cases.txt \
  || { echo "regex-diff: the generator failed" >&2; exit 1; }
./build/regex-diff/compare
