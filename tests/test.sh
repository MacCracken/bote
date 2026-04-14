#!/bin/sh
CC="${1:-./build/cc3}"
echo "=== bote tests ==="
cat src/main.cyr | "$CC" > /tmp/bote_test && chmod +x /tmp/bote_test && /tmp/bote_test
echo "exit: $?"
rm -f /tmp/bote_test
