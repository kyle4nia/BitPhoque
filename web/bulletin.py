#!/usr/bin/env python3
"""BitPhoque listing bulletin board -- minimal stdlib-only HTTP service.

GET  /api/listings          -> JSON array of all listings
POST /api/listings          -> add a listing (body: {"descriptor": "...", "txid": "..."})
POST /api/listings/sold     -> mark a listing sold
GET  /api/parcels?owner=KEY -> JSON array of parcels for that owner
POST /api/parcels           -> register parcels (body: {"parcels": [...]})
POST /api/support           -> save support ticket (body: {"pubkey": "...", "message": "..."})
GET  /api/health            -> {"status": "ok"}

Storage: flat JSON files on disk, rewritten on every POST.
Sits behind Caddy (TLS termination). Listens on 127.0.0.1:8500.
"""

import json
import os
import sys
import time
import urllib.request
import urllib.parse
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

LISTEN = "127.0.0.1"
PORT = 8500
DATA_FILE = "/root/bulletin/listings.json"
PARCELS_FILE = "/root/bulletin/parcels.json"
SUPPORT_FILE = "/root/bulletin/support.json"
TELEGRAM_CONFIG_FILE = "/root/bulletin/telegram_config.json"
MAX_LISTINGS = 1000  # drop oldest on overflow
MAX_PARCELS = 50000  # generous cap
MAX_BODY = 4096      # reject absurdly large POSTs
MAX_PARCELS_BODY = 65536  # parcels POST can be larger (batch registration)
SOLD_RETENTION_SECONDS = 7 * 24 * 3600  # keep sold listings visible 1 week, then prune
SUPPORT_RATE_LIMIT = 5   # max per pubkey per hour
SUPPORT_MAX_BODY = 4096

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

def load_parcels():
    if not os.path.exists(PARCELS_FILE):
        return []
    try:
        with open(PARCELS_FILE, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return []

def save_parcels(parcel_list):
    tmp = PARCELS_FILE + ".tmp"
    with open(tmp, "w") as f:
        json.dump(parcel_list, f)
    os.replace(tmp, PARCELS_FILE)

def load_support():
    if not os.path.exists(SUPPORT_FILE):
        return []
    try:
        with open(SUPPORT_FILE, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return []

def save_support(tickets):
    tmp = SUPPORT_FILE + ".tmp"
    with open(tmp, "w") as f:
        json.dump(tickets, f, indent=2)
    os.replace(tmp, SUPPORT_FILE)

def load_telegram_config():
    if not os.path.exists(TELEGRAM_CONFIG_FILE):
        return None
    try:
        with open(TELEGRAM_CONFIG_FILE, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return None

def send_telegram(text):
    """Send a message via Telegram bot. Fire-and-forget -- never blocks the support response."""
    if not telegram_config:
        return
    try:
        url = "https://api.telegram.org/bot" + telegram_config["bot_token"] + "/sendMessage"
        data = urllib.parse.urlencode({"chat_id": telegram_config["chat_id"], "text": text, "parse_mode": "HTML"}).encode()
        req = urllib.request.Request(url, data=data, method="POST")
        urllib.request.urlopen(req, timeout=5)
    except Exception as e:
        sys.stderr.write(f"[bulletin] Telegram send failed: {e}\n")

def prune_sold_listings():
    global listings
    now = int(time.time())
    kept = [l for l in listings if not (l.get("sold_at") and now - l["sold_at"] > SOLD_RETENTION_SECONDS)]
    if len(kept) != len(listings):
        listings = kept
        save_listings(listings)

listings = load_listings()
parcels = load_parcels()
telegram_config = load_telegram_config()
support_rate = {}  # pubkey -> [timestamps]

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
        parsed = urlparse(self.path)
        if parsed.path == "/api/listings":
            prune_sold_listings()
            self._json_response(200, listings)
        elif parsed.path == "/api/parcels":
            self._handle_get_parcels(parsed)
        elif parsed.path == "/api/health":
            self._json_response(200, {"status": "ok", "listings": len(listings), "parcels": len(parcels)})
        else:
            self._json_response(404, {"error": "not found"})

    def do_POST(self):
        if self.path == "/api/listings":
            self._handle_post_listing()
        elif self.path == "/api/listings/sold":
            self._handle_post_sold()
        elif self.path == "/api/parcels":
            self._handle_post_parcels()
        elif self.path == "/api/support":
            self._handle_post_support()
        else:
            self._json_response(404, {"error": "not found"})

    def _handle_get_parcels(self, parsed):
        qs = parse_qs(parsed.query)
        owner = qs.get("owner", [None])[0]
        if not owner:
            self._json_response(400, {"error": "missing owner param"})
            return
        result = [p for p in parcels if p.get("owner") == owner]
        self._json_response(200, result)

    def _handle_post_parcels(self):
        global parcels
        length = int(self.headers.get("Content-Length", 0))
        if length > MAX_PARCELS_BODY:
            self._json_response(413, {"error": "too large"})
            return

        try:
            raw = self.rfile.read(length)
            body = json.loads(raw)
        except (json.JSONDecodeError, Exception):
            self._json_response(400, {"error": "bad json"})
            return

        items = body.get("parcels", [])
        if not isinstance(items, list) or len(items) == 0:
            self._json_response(400, {"error": "missing or empty parcels array"})
            return

        # build set of known outpoints for fast dedup
        known = set()
        for p in parcels:
            known.add(p.get("txid", "") + ":" + str(p.get("index", "")))

        added = 0
        for item in items:
            txid = str(item.get("txid", "")).strip()
            index = item.get("index", 0)
            outpoint = txid + ":" + str(index)
            if outpoint in known:
                continue
            entry = {
                "owner": str(item.get("owner", "")).strip(),
                "amount": str(item.get("amount", "0")),
                "forSale": bool(item.get("forSale", False)),
                "price": str(item.get("price", "0")),
                "txid": txid,
                "index": int(index),
            }
            if not entry["owner"] or not entry["txid"]:
                continue
            parcels.append(entry)
            known.add(outpoint)
            added += 1

        # cap overflow -- drop oldest
        while len(parcels) > MAX_PARCELS:
            parcels.pop(0)

        if added > 0:
            save_parcels(parcels)

        self._json_response(201, {"status": "registered", "added": added, "total": len(parcels)})

    def _handle_post_support(self):
        global support_rate
        length = int(self.headers.get("Content-Length", 0))
        if length > SUPPORT_MAX_BODY:
            self._json_response(413, {"error": "too large"})
            return

        try:
            raw = self.rfile.read(length)
            body = json.loads(raw)
        except (json.JSONDecodeError, Exception):
            self._json_response(400, {"error": "bad json"})
            return

        pubkey = str(body.get("pubkey", "")).strip()
        message = str(body.get("message", "")).strip()


        if not pubkey or not message:
            self._json_response(400, {"error": "missing pubkey or message"})
            return

        # rate limit: 5 per pubkey per hour
        now = time.time()
        cutoff = now - 3600
        if pubkey in support_rate:
            support_rate[pubkey] = [t for t in support_rate[pubkey] if t > cutoff]
        else:
            support_rate[pubkey] = []

        if len(support_rate[pubkey]) >= SUPPORT_RATE_LIMIT:
            self._json_response(429, {"error": "rate limit exceeded, try again later"})
            return

        # save to local support log
        entry = {
            "pubkey": pubkey,
            "message": message,
            "timestamp": int(time.time()),
            "time_str": time.strftime("%Y-%m-%d %H:%M:%S UTC", time.gmtime()),
        }
        try:
            tickets = load_support()
            tickets.append(entry)
            save_support(tickets)
            support_rate[pubkey].append(now)
            self.log_message("SUPPORT: ticket saved for pubkey %s", pubkey[:12])

            # notify via Telegram
            tg_text = (
                "<b>BitPhoque Support</b>\n\n"
                "<b>Key:</b> <code>" + pubkey[:16] + "...</code>\n"
                "<b>Message:</b> " + message[:500] + "\n\n"
                "<i>" + entry["time_str"] + "</i>"
            )
            send_telegram(tg_text)

            self._json_response(200, {"status": "sent"})

        except Exception as e:
            self.log_message("SUPPORT: save error: %s", str(e))
            self._json_response(500, {"error": "failed to save support ticket"})

    def _handle_post_listing(self):
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

        # duplicate check -- same descriptor already listed
        for existing in listings:
            if existing.get("descriptor") == descriptor:
                self._json_response(200, {"status": "already listed"})
                return

        entry = {
            "descriptor": descriptor,
            "txid": txid,
            "timestamp": int(time.time()),
            "sold_at": None,
        }
        listings.append(entry)

        # cap overflow -- drop oldest
        while len(listings) > MAX_LISTINGS:
            listings.pop(0)

        save_listings(listings)
        self._json_response(201, {"status": "listed", "count": len(listings)})

    def _handle_post_sold(self):
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
        if not descriptor:
            self._json_response(400, {"error": "missing descriptor"})
            return

        for existing in listings:
            if existing.get("descriptor") == descriptor:
                if not existing.get("sold_at"):
                    existing["sold_at"] = int(time.time())
                    save_listings(listings)
                self._json_response(200, {"status": "sold", "sold_at": existing["sold_at"]})
                return

        self._json_response(404, {"error": "listing not found"})

    def do_DELETE(self):
        self._json_response(405, {"error": "not implemented"})

if __name__ == "__main__":
    os.makedirs(os.path.dirname(DATA_FILE), exist_ok=True)
    os.makedirs(os.path.dirname(PARCELS_FILE), exist_ok=True)
    print(f"[bulletin] serving on {LISTEN}:{PORT}, data: {DATA_FILE}, parcels: {PARCELS_FILE}")
    print(f"[bulletin] support tickets: {SUPPORT_FILE}")
    if telegram_config:
        print("[bulletin] Telegram notifications: enabled")
    else:
        print("[bulletin] WARNING: telegram_config.json not found, no notifications")
    server = HTTPServer((LISTEN, PORT), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[bulletin] shutting down.")
