#!/bin/bash
# bote against the official MCP client's own validator on `unevaluatedProperties` and
# `unevaluatedItems`: random schemas built around them and every keyword that evaluates a member or
# an item beside them, and random objects and arrays, each schema compiled and each value judged by
# the client's AjvJsonSchemaValidator (the Ajv it bundles), and the form 2025-11-25 lists a
# non-object root in made by the SDK's own wrapOutputSchemaForLegacy, taken from the client's
# bundle. The comparison is tests/fixtures/dynref_diff.cyr's: it fails on a schema bote takes that
# the client cannot compile, on a 2025-11-25 form that differs from the SDK's, and on a value bote
# judges otherwise than the client; a schema bote refuses that the client takes is counted, and
# registration says why.
#
# Not a CI step: it needs node and the client, which ./scripts/conformance.sh installs under
# build/conformance. Run it by hand at a release that touches how src/schema.cyr reads what is
# evaluated. It writes only under build/uneval-diff.
#
#   ./scripts/uneval-diff.sh
set -u
cd "$(dirname "$0")/.." || exit 1
command -v node >/dev/null 2>&1 || { echo "uneval-diff: needs node" >&2; exit 1; }
client="$PWD/build/conformance/node_modules/@modelcontextprotocol/client/dist/validators/ajv.cjs"
[ -f "$client" ] || { echo "uneval-diff: needs the client; run ./scripts/conformance.sh once" >&2; exit 1; }
mkdir -p build/uneval-diff
cyrius build tests/fixtures/dynref_diff.cyr build/uneval-diff/compare >/dev/null 2>&1 \
  || { echo "uneval-diff: the comparison program does not build" >&2; exit 1; }
{ for seed in 11 12 13 14 15; do node tests/fixtures/uneval_diff_gen.cjs "$client" "$seed" 20000 8 || exit 1; done; } > build/uneval-diff/cases.txt \
  || { echo "uneval-diff: the generator failed" >&2; exit 1; }
./build/uneval-diff/compare build/uneval-diff
