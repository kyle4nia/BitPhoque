#!/usr/bin/env python3
"""BitPhoque listing bulletin board — minimal stdlib-only HTTP service.

GET  /api/listings          → JSON array of all listings
POST /api/listings          → add a listing (body: {"descriptor": "...", "txid": "..."})
GET  /api/health            → {"status": "ok"}

Storage: flat JSON file on disk, rewritten on every POST.
Sits behind Caddy (TLS termination). Listens on 127.0.0.1:8500.
"""

import json
import os
import sys
import time
from http.server import HTTPServer, BaseHTTPRequestHandler

LISTEN = "127.0.0.1"
PORT = 8500
DATA_FILE = "/root/bulletin/listings.json"
MAX_LISTINGS = 1000  # drop oldest on overflow
MAX_BODY = 4096      # reject absurdly large POSTs

def load_listings():
    if not os.path.exists(DATA_FILE):
        return []
    try:
        with open(DATA_FILE, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return []

def save_listings(listings):
    tmp = DATA_FILE + ".tmp"
    with open(tmp, "w") as f:
        json.dump(listings, f)
    os.replace(tmp, DATA_FILE)

listings = load_listings()

class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        ts = time.strftime("%Y-%m-%d %H:%M:%S")
        sys.stderr.write(f"[bulletin {ts}] {fmt % args}\n")

    def _cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")

    def _json_response(self, code, obj):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self._cors()
        self.end_headers()
        self.write(body)

    def write(self, data):
        try:
            self.wfile.write(data)
        except BrokenPipeError:
            pass

    def do_OPTIONS(self):
        self.send_response(204)
        self._cors()
        self.end_headers()

    def do_GET(self):
        if self.path == "/api/listings":
            self._json_response(200, listings)
        elif self.path == "/api/health":
            self._json_response(200, {"status": "ok", "count": len(listings)})
        else:
            self._json_response(404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/api/listings":
            self._json_response(404, {"error": "not found"})
            return

        length = int(self.headers.get("Content-Length", 0))
        if length > MAX_BODY:
            self._json_response(413, {"error": "too large"})
            return

        try:
            raw = self.rfile.read(length)
            body = json.loads(raw)
        except (json.JSONDecodeError, Exception):
            self._json_response(400, {"error": "bad json"})
            return

        descriptor = body.get("descriptor", "").strip()
        txid = body.get("txid", "").strip()

        if not descriptor:
            self._json_response(400, {"error": "missing descriptor"})
            return

        # duplicate check — same descriptor already listed
        for existing in listings:
            if existing.get("descriptor") == descriptor:
                self._json_response(200, {"status": "already listed"})
                return

        entry = {
            "descriptor": descriptor,
            "txid": txid,
            "timestamp": int(time.time()),
        }
        listings.append(entry)

        # cap overflow — drop oldest
        while len(listings) > MAX_LISTINGS:
            listings.pop(0)

        save_listings(listings)
        self._json_response(201, {"status": "listed", "count": len(listings)})

    def do_DELETE(self):
        self._json_response(405, {"error": "not implemented — lazy approach, remember?"})

if __name__ == "__main__":
    os.makedirs(os.path.dirname(DATA_FILE), exist_ok=True)
    print(f"[bulletin] serving on {LISTEN}:{PORT}, data: {DATA_FILE}")
    server = HTTPServer((LISTEN, PORT), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[bulletin] shutting down.")
