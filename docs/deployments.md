# BitPhoque — Deployments

_WHICH: append-only registry of every deployed covenant lineage (cid, template, addresses, genesis tx, key anchors). Exists so live covenant identity isn't scattered across ledger sigs. Lookup only; secrets never recorded here._

---

## TN10 exercise family — Argent KCC20 standalone (2026-10-15)

First end-to-end on-chain exercise of the Argent / KCC20 BitPhoque on TN10. Throwaway family (fresh faucet-funded keys, no production value); proves genesis + the full sale lifecycle + a payment-authorized claim against the live node (VPS, reached via the 16210 tunnel).

- **Asset:** standalone KCC20 — app `BitPhoque { actor KCC20 }` (`contracts/bitphoque_app.ag`), fixed supply 42,069 phoques, pubkey-owned (owner_scheme 0x00), no minter.
- **covenant_id:** `1f5c88a3357f946ffd8fe803e2c1021f645b2e72eb2ef4d2a4f8f1798976f11d`
- **KCC20 template_hash:** `0ad39f46e707adda3a647aae30db95d88cb83bec0af669663789684b8e12c225`
- **Node:** Hetzner VPS TN10, kaspad v2.0.1, gRPC via tunnel `127.0.0.1:16210`.
- **Pins:** argent `9a9f4b10`, silverscript `3ed9733` (v1.0.0), rusty-kaspa `a41a333b`.

### Keys (secrets gitignored on pyt; addresses public)
- **Seller / owner** (`exercise_funding.key`): `kaspatest:qr7cx3v7ngcskr2wclx8tscwx2zrdldqleds7hwhc6807adzwz75jewuhrlvl`
- **Buyer** (`exercise_buyer.key`): `kaspatest:qrd09h9at34sy0a7mswg6lm3046aslvaecunyeeqyuqgsy8xpt64wc4zv7mkq`

### On-chain tx chain
| step | txid | note |
|---|---|---|
| genesis | `044c41798b7fcdc9d97c17957335cc717d50a51032d2d6d798b29ed0bf79fe2d` | 42,069 minted idle, 10 TKAS gas tank; idle addr `kaspatest:pznu0vgl…9xpdyps` |
| list 0→500 | `6430d76c72c95057424be2694188cd83ddeb584d673f829a7dd6de0315770620` | idle → price 500 |
| reprice 500→1200 | `a8964eedab97342ddcbbb4089f21fa7250527648e85c7f4958c2b5b28b35a615` | |
| cancel 1200→0 | `3318ed23ef20d264ee53f7de664577116d577b62a73a6090ef7bd7f932f145e2` | back to idle addr (deterministic) |
| relist 0→100000000 | `c36e6fda381ea3ccb9f5a026a398fe5bbcf8ab3122ee97630b1515758de6c9f9` | listed at 1 TKAS for claim |
| claim | `e9abe06bdd26a5be918c0e5cafb87bbef94cef6fab5c69946a80a7b7d83f8064` | buyer paid seller 1 TKAS, took token (NO seller sig) |

### Current live state
- Token: **buyer-owned, idle**, outpoint `e9abe06b…:0`, 996,000,000 sompi, covenant_id preserved, at `kaspatest:pzgdfgvu3jm8pg94k8xgxcg9sex6tv9dd97qshds72kwkpnheesqg7d7qw3n9`.
- Seller payment UTXO: `e9abe06b…:1` = 100,000,000 sompi.

### Confirmed by this exercise
- Argent-derived P2SH + covenant_id match the node's index (genesis "verify by actual import" satisfied).
- Price lives in covenant state → each price is its own P2SH address (per-commitment-class, per DESIGN).
- `set_price` owner-authorized + singleton; `cancel` restores the exact genesis/idle address.
- `claim` payment-authorized (no seller sig) + singleton; claimed token forced idle + borrow_guard zeroed (sig #024); owner changes to buyer (new idle address).

### Tooling (bitphoque-argent bins, gRPC via 16210 tunnel; PROTOC needed to build)
`daginfo` · `utxos [addr]` · `fundaddr [keyfile]` · `genesis [--go]` · `setprice <from> <to> [--go]` · `claim <price> [--go]`
