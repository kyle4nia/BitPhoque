# BitPhoque Session Handoff -- 2026-06-28

_Replaces `bitphoque_ui_session_continuation.md`. Fold into the status doc
at next session start, then delete this file._

---

## Infrastructure

**VPS upgraded: CX22 -> CX33 (Hetzner, Helsinki)**
- Was: 4GB RAM / 2 vCPU / 40GB disk. kaspad OOM-killed repeatedly during
  IBD -- 65 restart cycles, never completed sync.
- Now: 8GB RAM / 4 vCPU / 80GB disk. 4GB swap left in place as safety net.
  IBD restarted from fresh datadir (`/root/.kaspad` wiped pre-upgrade).
  kaspad was syncing headers steadily at ~1500/10s as of session end.
  IP unchanged: `89.167.32.239`. DNS/Caddy/systemd service unchanged.
- **Check on next session:** Has IBD completed? Look for nonzero
  "UTXO-validated blocks" and "Processed N blocks" in recent logs.
  `ssh root@89.167.32.239 "journalctl -u kaspad --no-pager -n 10"`

---

## UI/UX changes (committed as 2c4f55c)

All changes are in `web/` and deployed by pushing to cPanel
(`public_html/BitPhoque/`).

### wallet.html
- Seal size: 46 -> 68px (`.seal` + `.seal-fallback`)
- Welcome screen title + lede centered (`#s-welcome` scoped CSS)
- "Buy me a coffee" footer with QR code (kaspa address inline)
- "Learn about this project" collapsible `<details>` section on home screen:
  three pillars elaborated (self-custody, fixed supply, on Kaspa) + Oracle
  Protocol blurb with GitHub link + "Back to BitPhoque.org" link
- Receive panel: shows x-only pubkey + QR instead of kaspatest address.
  `pubkeyField: true` replaces `addrField: true`. `copyPubkey()` added.

### index.html
- Tagline: "all out of phoques to give" -> "All out of phoques? Let's fix that."
- Lander logo: 168 -> 210px (25% increase), fallback matched
- "Open wallet" button plays `phoque3.wav` -> `phoque2.wav` (overlap=2.0s
  to eat trailing silence in phoque3) before navigating to wallet.html
- "Learn about this project" collapsible section (same content as wallet)
- "Buy me a coffee" footer with QR code

### New files added to tree
```
web/bitphoque_qr.js              <- QR SVG wrapper, exports qrSvg(text, size)
web/vendor/qrcode-generator.mjs  <- vendored MIT library (Kazuhiko Arase, ~52KB)
web/phoque2.wav                  <- second audio file for lander button
```

### cPanel upload checklist
Push to `public_html/BitPhoque/`:
- `index.html` (updated)
- `wallet.html` (updated)
- `bitphoque_qr.js` (new)
- `vendor/qrcode-generator.mjs` (new)
- `phoque2.wav` (new -- Kyle added during session)

---

## Wallet v2 design spec

A full design spec was written and saved at `docs/bitphoque_wallet_v2_design.md`.

**The core shift:** The wallet stops being six task panels and becomes a
state-driven view of your phoques. Cards show each group of phoques with
its on-chain state (held, for sale, incoming, sending). Actions are
contextual on cards, not chosen from a menu. The words "split," "merge,"
"descriptor," "UTXO," "P2SH," and "sighash" never appear in the UI.

**Four user stories replace six panels:**
1. Send (handles split silently)
2. Receive (scan/paste a code, wallet claims)
3. Sell (list from a held card)
4. Buy (scan a listing)

**Mockups were sketched in-chat** for: home screen (parcel cards), send
flow (auto-split), receive flow (QR scan + claim). Not saved as files --
the design spec captures the decisions; re-sketch from it.

**Open design questions** (settle before building):
- Auto-claim incoming parcels vs manual "Claim" tap?
- One card per parcel vs one card per state (aggregated)?
- Multiple parcels in a single send (merge)?
- Descriptor relay (post-testnet infrastructure)?

---

## Open items

### Ready to build
- [ ] Wallet v2 UI rebuild (from `docs/bitphoque_wallet_v2_design.md`)
- [ ] Wallet usage instructions collapsible (step-by-step for each action)
- [ ] Per-panel info links (quiet help icon per action, context-sensitive)

### Needs scoping
- [ ] Automated maintenance / GitHub alerts / deployment automation
- [ ] Descriptor relay service (post-testnet)

### Parked
- [ ] Better tagline than "a seal of a token"
- [ ] General panel UX improvements (subsumed by v2 redesign)

---

## Git state at session end

Commit `2c4f55c` covers the UI/UX pass. Two files remain uncommitted:
- `docs/bitphoque_wallet_v2_design.md` (design spec)
- This handoff file (replaces `docs/bitphoque_ui_session_continuation.md`)

**Start of next session:** commit the design spec, replace the old
continuation file with this one (or delete both after folding into status
doc), verify IBD status on VPS.
