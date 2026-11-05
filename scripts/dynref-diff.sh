#!/bin/bash
# bote against the official MCP client's own validator on references, static and dynamic: random
# schemas built around `$ref`, `$dynamicRef`, `$recursiveRef`, their anchors and `$id`s, and random
# values, each schema compiled and each value judged by the client's AjvJsonSchemaValidator (the
# Ajv it bundles), and the form 2025-11-25 lists a non-object root in made by the SDK's own
# wrapOutputSchemaForLegacy, taken from the client's bundle. It fails on a schema bote takes that
# the client cannot compile, on a 2025-11-25 form that differs from the SDK's, and on a value
# bote judges otherwise than the client; a schema bote refuses that the client takes is counted,
# and registration says why.
#
# Not a CI step: it needs node and the client, which ./scripts/conformance.sh installs under
# build/conformance. Run it by hand at a release that touches how src/schema.cyr reads or
# judges a reference. It writes only under build/dynref-diff. CI builds the comparison program,
# so it does not rot.
#
#   ./scripts/dynref-diff.sh
set -u
cd "$(dirname "$0")/.." || exit 1
command -v node >/dev/null 2>&1 || { echo "dynref-diff: needs node" >&2; exit 1; }
client="$PWD/build/conformance/node_modules/@modelcontextprotocol/client/dist/validators/ajv.cjs"
[ -f "$client" ] || { echo "dynref-diff: needs the client; run ./scripts/conformance.sh once" >&2; exit 1; }
mkdir -p build/dynref-diff
cyrius build tests/fixtures/dynref_diff.cyr build/dynref-diff/compare >/dev/null 2>&1 \
  || { echo "dynref-diff: the comparison program does not build" >&2; exit 1; }
{ for seed in 3 4 5 6 7; do node tests/fixtures/dynref_diff_gen.cjs "$client" "$seed" 20000 8 || exit 1; done; } > build/dynref-diff/cases.txt \
  || { echo "dynref-diff: the generator failed" >&2; exit 1; }
./build/dynref-diff/compare
