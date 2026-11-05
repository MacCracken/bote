#!/bin/bash
# bote's `format` (src/format.cyr) against the official MCP client's own validator: about half
# a million strings and numbers near each format's edges, seeds the client takes mutated and
# strings drawn from each format's alphabet, each judged by the client's AjvJsonSchemaValidator
# (the Ajv and ajv-formats it bundles) and by bote. It fails on any string the two judge apart
# and any number bote takes that the client refuses; a number bote refuses exactly that the
# client's double takes (int32's 5.0000000000000001) is counted.
#
# Not a CI step: it needs node and the client, which ./scripts/conformance.sh installs under
# build/conformance. Run it by hand at a release that touches src/format.cyr or src/regex.cyr.
# It writes only under build/format-diff. CI builds the comparison program, so it does not rot.
#
#   ./scripts/format-diff.sh
set -u
cd "$(dirname "$0")/.." || exit 1
command -v node >/dev/null 2>&1 || { echo "format-diff: needs node" >&2; exit 1; }
client="$PWD/build/conformance/node_modules/@modelcontextprotocol/client/dist/validators/ajv.cjs"
[ -f "$client" ] || { echo "format-diff: needs the client; run ./scripts/conformance.sh once" >&2; exit 1; }
mkdir -p build/format-diff
cyrius build tests/fixtures/format_diff.cyr build/format-diff/compare >/dev/null 2>&1 \
  || { echo "format-diff: the comparison program does not build" >&2; exit 1; }
{ for seed in 5 7 11 13; do node tests/fixtures/format_diff_gen.cjs "$client" "$seed" 4000 || exit 1; done; } > build/format-diff/cases.txt \
  || { echo "format-diff: the generator failed" >&2; exit 1; }
./build/format-diff/compare
