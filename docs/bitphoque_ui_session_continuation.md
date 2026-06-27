# BitPhoque UI Session — 2026-06-27

Pick up from here. Status doc (`bitphoque_status.md`) is unchanged — this session was
cosmetic/UX with two functional additions.

## What was done

### Layout & cosmetic (wallet.html + index.html)
- Seal size: wallet header 46→68px, lander logo 168→210px (25% bump)
- Welcome screen title + lede centered
- Lander tagline changed: "all out of phoques to give" → "All out of phoques? Let's fix that."
- "Buy me a coffee" footer on both pages with QR code
  - Address: `kaspa:qrwdl4f0snyfe66zjd8np8mrls7qf8t8le0zcdrmqww97uqu9ay6vjvlt6qkl`
- "Learn about this project" collapsible section on both pages (three pillars + Oracle Protocol)
- Lander "Open wallet" button plays `phoque3.wav` → `phoque2.wav` (overlap=2.0s) before navigating

### Functional changes
- **Receive panel: pubkey instead of address.** Shows x-only pubkey + QR code + "Copy pubkey"
  button. The kaspatest address was misleading (amount-derived P2SH means no single address
  represents the wallet). The sender needs the pubkey, not an address. `pubkeyField: true`
  replaces `addrField: true` on the receive panel; `copyPubkey()` added alongside `copyAddr()`.
- **QR code infrastructure.** New files:
  - `web/bitphoque_qr.js` — thin wrapper, exports `qrSvg(text, size)` returning inline SVG
  - `web/vendor/qrcode-generator.mjs` — vendored MIT library (Kazuhiko Arase, ~52KB)
  - Used in: coffee footer (both pages), receive panel pubkey QR

### New files added to tree
```
web/bitphoque_qr.js              <- QR SVG generator wrapper
web/vendor/qrcode-generator.mjs  <- vendored QR library
```

### cPanel upload needed
Push these to `public_html/BitPhoque/`:
- `index.html` (updated)
- `wallet.html` (updated)
- `bitphoque_qr.js` (new)
- `vendor/qrcode-generator.mjs` (new)
- `phoque2.wav` (new — Kyle already added to cPanel during session)

## Open items (new from this session)

- [ ] **Wallet usage instructions dropdown** — second collapsible section on the wallet
      home screen with step-by-step instructions for each operation (send, receive, claim,
      list, split, merge)
- [ ] **Per-panel info links** — quiet info/help icon or link on each action panel that
      expands instructions specific to that task
- [ ] **Automated maintenance / GitHub alerts** — GitHub Actions or tooling for update
      alerts, automated checks, deployment automation. Needs scoping: what exactly to
      monitor (node health? repo pushes? dependency updates?)
- [ ] Item 7 (parked): better tagline than "a seal of a token"
- [ ] Item 9 (TBD): general panel UX improvements

## Status doc update needed?

Minimal — when next session starts, add to the FILE TREE section:
- `bitphoque_qr.js` and `vendor/qrcode-generator.mjs`
Note the receive-panel pubkey change in the "Panels wired" summary.
Then delete this continuation file (it's a bridge, not a living doc).
