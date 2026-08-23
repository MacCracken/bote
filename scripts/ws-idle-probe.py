#!/usr/bin/env python3
# ws-idle-probe.py — runtime proof for the 3.3.7 WebSocket idle-drop fix.
#
#   ./build/bote-ws 8399 &
#   python3 scripts/ws-idle-probe.py 8399 3    # control: expect ALIVE  (exit 0)
#   python3 scripts/ws-idle-probe.py 8399 33   # expect ALIVE after the fix (exit 0)
#                                              # expected DROPPED before it (exit 1)
#
# Why this exists as a script rather than a .tcyr assertion: the defect is a
# wall-clock effect on a real socket (sandhi's 30 s SO_RCVTIMEO leaking into
# the frame loop), so it cannot be observed in-process.
#
# ⚠ THE DRAIN IS LOAD-BEARING. The first version of this probe read only
# `recv(4096)` once after the pre-idle request and got 4 of 138 bytes. Its
# post-idle read then returned the ~134 STALE BUFFERED BYTES still sitting in
# the local socket buffer — so a dropped connection looked ALIVE and the bug
# "did not reproduce". Any rewrite must drain to a timeout before idling, or
# it will pass while the bug is present.
import socket, base64, os, sys, time
PORT=int(sys.argv[1]); IDLE=float(sys.argv[2])
def frame(p):
    b=p.encode(); m=os.urandom(4); o=bytearray([0x81]); o.append(0x80|len(b)); o+=m
    o+=bytes(c^m[i%4] for i,c in enumerate(b)); return bytes(o)
def drain(s):
    """Read until the peer has nothing more queued right now."""
    s.settimeout(1.5); total=0
    while True:
        try:
            d=s.recv(65536)
            if not d: return total, True          # EOF
            total+=len(d)
        except socket.timeout:
            return total, False
s=socket.create_connection(("127.0.0.1",PORT),timeout=10)
k=base64.b64encode(os.urandom(16)).decode()
s.sendall((f"GET /mcp HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
           f"Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {k}\r\n\r\n").encode())
s.settimeout(5); r=s.recv(4096)
assert b"101" in r.split(b"\r\n")[0], r[:120]
print("handshake OK")
s.sendall(frame('{"jsonrpc":"2.0","id":1,"method":"tools/list"}'))
n,eof=drain(s); print(f"pre-idle: drained {n} bytes, eof={eof}")
print(f"idling {IDLE}s (socket fully drained) ...")
time.sleep(IDLE)
try:
    s.sendall(frame('{"jsonrpc":"2.0","id":2,"method":"tools/list"}'))
    n2,eof2=drain(s)
    if eof2 or n2==0:
        print(f"RESULT: DROPPED after {IDLE}s idle (eof={eof2}, {n2} bytes)"); sys.exit(1)
    print(f"RESULT: ALIVE — fresh {n2} bytes after {IDLE}s idle"); sys.exit(0)
except (BrokenPipeError,ConnectionResetError) as e:
    print(f"RESULT: DROPPED — {type(e).__name__}"); sys.exit(1)
