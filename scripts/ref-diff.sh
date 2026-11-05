#!/bin/bash
# bote's registration against the official MCP client's own validator on which `$ref`s are in
# their document: 100,000 random root `$id`s (well formed and not) and references through
# them, each compiled by the client's AjvJsonSchemaValidator (the Ajv it bundles) and checked
# by bote's registration (_sch_refs_why). It fails on a schema bote takes that the client cannot compile, which
# would leave every result of the tool unjudgeable by the client; a schema bote refuses that
# the client takes is counted, and registration says why.
#
# Not a CI step: it needs node and the client, which ./scripts/conformance.sh installs under
# build/conformance. Run it by hand at a release that touches how src/schema.cyr reads
# `$id` or `$ref`. It writes only under build/ref-diff. CI builds the comparison program, so
# it does not rot.
#
#   ./scripts/ref-diff.sh
set -u
cd "$(dirname "$0")/.." || exit 1
command -v node >/dev/null 2>&1 || { echo "ref-diff: needs node" >&2; exit 1; }
client="$PWD/build/conformance/node_modules/@modelcontextprotocol/client/dist/validators/ajv.cjs"
[ -f "$client" ] || { echo "ref-diff: needs the client; run ./scripts/conformance.sh once" >&2; exit 1; }
mkdir -p build/ref-diff
cyrius build tests/fixtures/ref_diff.cyr build/ref-diff/compare >/dev/null 2>&1 \
  || { echo "ref-diff: the comparison program does not build" >&2; exit 1; }
{ for seed in 7 8 21 22 23; do node tests/fixtures/ref_diff_gen.cjs "$client" "$seed" 20000 || exit 1; done; } > build/ref-diff/cases.txt \
  || { echo "ref-diff: the generator failed" >&2; exit 1; }
./build/ref-diff/compare
