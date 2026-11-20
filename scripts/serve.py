#!/usr/bin/env python3
"""Serve only the web build on localhost, with the correct WebAssembly MIME type."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int, default=8086)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent / "web"
SimpleHTTPRequestHandler.extensions_map[".wasm"] = "application/wasm"
server = ThreadingHTTPServer(("127.0.0.1", args.port), partial(SimpleHTTPRequestHandler, directory=str(root)))
print(f"Play Rocketpult: http://127.0.0.1:{args.port}", flush=True)
try:
    server.serve_forever()
except KeyboardInterrupt:
    server.server_close()
