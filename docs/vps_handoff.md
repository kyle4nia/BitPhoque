# BitPhoque VPS Node -- Handoff

_Created 2026-06-27. This doc captures the Hetzner VPS setup for the public
TN10 node that serves the BitPhoque web wallet._

---

## What was done this session

1. **Hetzner VPS provisioned** -- CX22 (2 vCPU, 4GB RAM, 40GB disk), Helsinki,
   Ubuntu 24.04. IP: `89.167.32.239`.

2. **kaspad v2.0.1 installed** -- downloaded from
   `github.com/kaspanet/rusty-kaspa/releases/download/v2.0.1/rusty-kaspa-v2.0.1-linux-amd64.zip`,
   unpacked to `/root/bin/kaspad`. Same post-Toccata release as Big Daddy.

3. **kaspad running as systemd service** -- auto-starts on reboot.
   - Service file: `/etc/systemd/system/kaspad.service`
   - Launch flags: `--testnet --netsuffix=10 --utxoindex --rpclisten-json=0.0.0.0:18210`
   - Data lives in default location (`/root/.kaspad/` or similar)
   - Commands: `systemctl status kaspad`, `systemctl restart kaspad`,
     `journalctl -u kaspad -f` (live logs)

4. **DNS A record added** -- `node.outlawpolitics.org` -> `89.167.32.239`,
   added via cPanel Zone Editor on `outlawpolitics.org`.

5. **Caddy reverse proxy** -- auto-provisions Let's Encrypt TLS cert.
   - Config: `/etc/caddy/Caddyfile`
   - Proxies `wss://node.outlawpolitics.org` -> `127.0.0.1:18210`
   - Commands: `systemctl status caddy`, `systemctl restart caddy`

6. **wallet.html updated** -- `WRPC_URL` changed from `ws://127.0.0.1:18210`
   to `wss://node.outlawpolitics.org`. Uploaded to cPanel.

7. **Full site published to cPanel** -- `public_html/BitPhoque/` contains
   index.html, wallet.html, favicon.ico, bitphoque-nowords.jpeg, phoque3.wav,
   all JS modules, and the vendor/ folder.

8. **Kernel updated + rebooted** -- running 7.0.0-27-generic, clean.

---

## What is still in progress

- **IBD (Initial Block Download)** -- kaspad was mid-sync at session end
  (validating pruning point proof, early phase). Until IBD completes and
  `synced=true`, the wallet's node calls (balance, submit) will fail.
  Check sync status: `ssh root@89.167.32.239`, then
  `journalctl -u kaspad --no-pager -n 20` and look for
  `synced=true` or block counts approaching the tip.

---

## Architecture

```
  Browser (anywhere)
    |
    | wss://node.outlawpolitics.org (port 443, encrypted)
    v
  Caddy (VPS, port 443, TLS termination, Let's Encrypt auto-renew)
    |
    | ws://127.0.0.1:18210 (local, plaintext)
    v
  kaspad (VPS, TN10 node, --utxoindex, --rpclisten-json)
```

The wallet static files are served by cPanel (outlawpolitics.org/BitPhoque/).
The node RPC is served by the Hetzner VPS. They are completely separate -- the
wallet JS makes WebSocket calls from the visitor's browser to the VPS node.

---

## Credentials / access

- **SSH:** `ssh root@89.167.32.239` (password in Hetzner dashboard)
- **Hetzner console:** console.hetzner.cloud (fallback if SSH breaks)
- **cPanel:** existing login for outlawpolitics.org

---

## Common operations

**Check if node is synced:**
```
ssh root@89.167.32.239
journalctl -u kaspad --no-pager -n 10
```
Look for `synced=true` or headers/blocks counts matching.

**Restart kaspad:**
```
systemctl restart kaspad
```

**Watch live kaspad logs:**
```
journalctl -u kaspad -f
```

**Restart Caddy:**
```
systemctl restart caddy
```

**Edit Caddy config:**
```
nano /etc/caddy/Caddyfile
systemctl restart caddy
```

---

## Mainnet swap checklist

When ready to move from TN10 to mainnet:

1. Stop kaspad: `systemctl stop kaspad`
2. Edit service file: `nano /etc/systemd/system/kaspad.service`
   - Remove `--testnet --netsuffix=10` from ExecStart
   - Keep `--utxoindex --rpclisten-json=0.0.0.0:18210`
3. Delete TN10 datadir (saves disk)
4. Reload + start: `systemctl daemon-reload && systemctl start kaspad`
5. Wait for mainnet IBD to complete
6. In wallet.html: change `KASPA.network` from `kaspatest` to `kaspa`
7. Re-genesis produces a new covenant_id -- sweep `3baed5a3...` in all JS
8. Re-upload wallet.html to cPanel

DNS and Caddy config stay the same -- `node.outlawpolitics.org` just starts
serving mainnet RPC instead of testnet.
