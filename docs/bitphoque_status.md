# BitPhoque — Status

_Read `operating_baseline.md` alongside this doc. BitPhoque lives in its own tree (`C:\bitphoque`)
and its own git repo — separate from oracle-protocol. End-of-session checkpoint covers both trees._

_The on-chain tx ledger (every confirmed txid) and session-by-session history live in
`bitphoque_history.md`. Start here; reach for history only to look up a past fact._

---

## WHERE WE ARE (2026-06-28)

The web wallet is a **spending wallet**. Every layer of the write seam is proven against ground
truth and live on TN10.

- **Onboarding:** BIP-39 12-word, real entropy (`@scure/bip39`), AES-GCM encrypted persistence,
  HD key derivation (`m/44'/111111'/0'/0/0` → secp256k1 x-only → Kaspa bech32), cross-checked vs
  KasWare.
- **Read side:** `refreshBalance()` — per-parcel P2SH derivation (template splice,
  `bitphoque_p2sh.js`) → `getUtxosByAddresses` over JSON wRPC (`wss://node.outlawpolitics.org`) →
  covenant filter → reconcile + prune.
- **Write side (PROVEN LIVE):** sighash (`bitphoque_sighash.js`) → schnorr sign (`@noble/curves`)
  → ABI sigscript (`bitphoque_sigscript.js`) → JSON submit via `wrpcCall("submitTransaction")`.
  **Live mined tx `4f3d791f`** — wallet sent 1000 phoques, node accepted.
- **Panels wired:** send, split, list, partial-list, claim, merge — all real build→sign→submit.
  Claim panel proven live (commit `70e9f96`): 3 txs (`d941a074`, `a8bc5081`, `77d595d8`).
  Merge panel proven live (commit `b04c041`).
- **CLI:** feature-complete — import / balance / list / split / merge / partial-list / claim /
  send / receive. All proven on-chain.
- **Audit:** `bitphoque_audit_2026-06-25.md` — no fund-loss or supply-break vector found.

### Wallet v2 — PUBLISHED (2026-06-28)

Wallet v2 replaces the original `wallet.html`. Card-based UI, no protocol jargon.
Sompi references removed from all user-facing labels (Buy panel price input now takes KAS,
converts internally). Published to cPanel at `outlawpolitics.org/BitPhoque/wallet.html`.

Prior commits covering UI/UX work: `2c4f55c` (seal resize, welcome screen, QR, collapsibles,
Receive panel pubkey+QR, audio), `39b8b88` (trademark on seal, powered-by-Kaspa banner).

**Deferred cleanup in `web\`:** `wallet_old.html` (pre-v2 backup) and `fix_price.ps1`
(one-shot edit script) — delete after v2 is tested.

**Wallet v2 design spec** at `docs/bitphoque_wallet_v2_design.md`. Open design questions:
auto-claim vs manual, card aggregation, multi-parcel send, descriptor relay.

### VPS + PUBLIC DEPLOY — DONE (2026-06-27)

**Hetzner CX33** (8GB RAM / 4 vCPU / 80GB, Helsinki, Ubuntu 24.04). IP: `89.167.32.239`.

```
  Browser (anywhere)
    |  wss://node.outlawpolitics.org (port 443, TLS)
    v
  Caddy (VPS, TLS termination, Let's Encrypt auto-renew)
    |  ws://127.0.0.1:18210 (local, plaintext)
    v
  kaspad v2.0.1 (VPS, TN10, --utxoindex, systemd service)
```

Wallet static files: cPanel (`outlawpolitics.org/BitPhoque/`).
Node RPC: Hetzner VPS. Completely separate.

SSH: `ssh root@89.167.32.239`. Full operational details in `vps_handoff.md`.

**IBD status:** was in progress at last check. Verify:
`journalctl -u kaspad --no-pager -n 20` → look for `synced=true`.

---

## What this is (one sentence)

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 — a practice run for
mainnet — built on silverscript covenants (`binding=cov` N-to-M plumbing), deliberately NOT the
indexer-based KRC-20 path. Phonetic play on "out of f**ks to give," seal mascot, fixed supply, no
trusted coordinator. Site: https://www.outlawpolitics.org/BitPhoque/

---

## DESIGN — SETTLED

- **Architecture:** per-UTXO balance (real token, not a single-UTXO ledger). Conservation-enforced.
- **Ownership:** per-UTXO signature, `IDENTIFIER_PUBKEY=0x00` (plain checkSig).
- **Supply:** FIXED at genesis, NO minter. 42,069 phoques. `genesisIsMinter=false`.
- **Subdivision — BitPups.** 1 phoque = 100,000,000 BitPups (8 dp). TN10 minted as whole units
  (needs re-genesis for mainnet sub-unit headroom).
- **Sale model (v3):** per-UTXO `forSale` + `price`. Listing = owner-signed. Claim =
  payment-authorized. F1 fix: strict-increasing payment-output indices.
- **Wallet custody:** non-custodial, BIP-39 12-word, key in-browser, never on server.
- **Receive:** off-chain signed parcel descriptor (118B, base58check) + on-chain confirm.
  Descriptor = claim, chain = truth.
- **KAS handling:** "gas-tank, not vault." Fee floors: 700k list/split, 900k claim, 1.3M merge.

---

## THE CONTRACTS

### Original token — `covenants/bitphoque.sil` (DEPLOYED)
Copied from `kcc20.sil`. State: ownerIdentifier + identifierType + amount + isMinter.
ABI: `__leader_transfer` + `__delegate_transfer`.

### Sale token v3 — `covenants/bitphoque_sale_v3.sil` (LIVE)
Extends with forSale/price. Script 2793 bytes.
covenant_id `3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458`.
Caps 3/3 baked at genesis. F1 fix: strict-increasing payment-output indices.

### Compile command
```
cargo run --bin silverc -- C:\bitphoque\covenants\<file>.sil --ctor C:\bitphoque\covenants\<ctor>.json -c
```

---

## CONSENSUS RULES

1. v1/Toccata txs use ComputeBudget for input mass, NOT SigopCount.
2. Covenant input: 111,261 script units → ComputeBudget(11). Plus ~100k for plain P2PK → also ComputeBudget(11).
3. KIP-9 dust penalty: pay non-dust amounts (≥100M sompi for claims).
4. Fee floors: genesis 200k; transfer 400k; list/split 700k; claim 900k; merge 1.3M; 2×2 claim 1.5M.
5. v1 needs ComputeBudget on EVERY input including plain P2PK.

### Claim ABI
- Witness byte `[1]` = payment output index.
- Dummy sig `[0u8; 65]` for payment-authorized branch.
- P2PK scriptPubKey: `20<32B xonly>ac`. Helper: `makeP2pk()`.
- computeBudget: 24 (covenant), 11 (plain P2PK).

### Leader/delegate ABI (multi-input)
- Input 0 = LEADER: carries newStates[], sigs[], witnesses[] for ALL covenant inputs.
- Other inputs = DELEGATE: empty args + redeem push only.
- ALL sigs computed against leader's sighash (index 0).

### Sigscript structure
State[] transposed: ownerIdentifier(32B), identifierType(1B), amount(8B LE), isMinter(1B),
forSale(1B), price(8B LE). Then sig[], witnesses, selector (OP_0/OP_1), redeem push (2793B).

### submitTransaction JSON format
```
wrpcCall("submitTransaction", {
  transaction: {
    version: 1,
    inputs: [{ previousOutpoint: {transactionId, index}, signatureScript (hex),
               sequence: 0, sigOpCount: 0, computeBudget: 24 }],
    outputs: [{ value, scriptPublicKey: "0000aa20<hash>87",
                covenant: {authorizingInput: 0, covenantId (hex)} }],
    lockTime: 0, subnetworkId: "00…00" (40 hex), gas: 0, payload: "", mass: 0
  },
  allowOrphan: false
})
```

### Node quirks
- Address-index lags tx-index by a few seconds. Mitigation: 3 retries, 3s apart.
- No by-outpoint UTXO query — `get_utxos_by_addresses` only, filter client-side.

---

## TN10 GROUND FACTS

- **Local node (Big Daddy):** `127.0.0.1:16210` gRPC, `18210` JSON wRPC. kaspad 2.0.1.
  Launch: `kaspad.exe v2.0.1 --testnet --netsuffix=10 --appdir=C:\kaspa-testnet10\datadir
  --utxoindex --disable-upnp --rpclisten=0.0.0.0:16210 --rpclisten-borsh=0.0.0.0:17210
  --rpclisten-json=0.0.0.0:18210`.
- **VPS node:** Hetzner CX33, `89.167.32.239`. `wss://node.outlawpolitics.org`.
  SSH: `ssh root@89.167.32.239`. Details in `vps_handoff.md`.
- **Build clone:** `D:\Tocatta_Projects\kaspa-tn12\silverscript`.
  **Recopy canonical→clone before running any harness from D:**.
- **NODE addr rule:** every `.rs` `const NODE` = `127.0.0.1:16210`. No machine IPs.

### Test entities (PUBLIC keys only)

| Entity | Label | Address | Pubkey (hex) | TKAS |
|--------|-------|---------|-------------|------|
| 1 | phoque (webapp) | `kaspatest:qzcpdfrm…` | `1602d48f…` | ~2,999 |
| 2 | kasware | same as Entity 1 (same seed) | same | ~2,999 |
| 3 | phonePhoques (phone) | `kaspatest:qzhl95s2…` | `15fe5a41…` | 1,000 |
| 4 | tabletphoques (tablet) | `kaspatest:qpx2a6yx…` | `0995dd10…` | 1,000 |

**Legacy key labels** (in harness source + `.key` files): buyer1 `8c02c20a…`, buyer2 `5d338aa5…`,
buyer3 `267496ce…`.

- **Faucet:** min 1000 TKAS/request, 24h cap 100k.

### Genesis vs parcel P2SH
Genesis = route-1 (`blake2b(entire compiled script)` → P2SH). Parcels = route-2 (template
splice → `blake2b` → P2SH). Different scripts, different addresses. Wallet derives route-2 only.

---

## KEY HISTORY (detail in `bitphoque_history.md`)

- **Toolchain split (2026-06-26):** 2-field vs 3-field `EngineFlags` mismatch. Fixed via
  `[patch]` block in `D:\…\silverscript\Cargo.toml` redirecting all kaspa crates to local
  3-field `rusty-kaspa`. Green-path proven: CLI send → web Receive → parcel confirmed.
- **Node addr sweep (2026-06-27):** all 14 `.rs` harnesses repointed from stale machine IPs
  to `127.0.0.1:16210`. Commit `e3f6628`.
- **VPS upgrade (2026-06-27):** CX22 → CX33 after kaspad OOM during IBD.
- **Claim harness EngineFlags fix (2026-06-27):** added `zk_hardening_enabled: true`.

---

## FILE TREE (`C:\bitphoque`)

```
C:\bitphoque\
  .gitignore                           <- *.key + wallet.json
  bitphoque_owner.key                  <- original/funding signer (GITIGNORED)
  bitphoque_buyer1.key                 <- buyer1 8c02c20a… (GITIGNORED)
  bitphoque_buyer2.key                 <- buyer2 5d338aa5… (GITIGNORED)
  bitphoque_buyer3.key                 <- buyer3 267496ce… (GITIGNORED)
  wallet.json                          <- CLI parcel state (GITIGNORED)
  covenants\
    bitphoque_sale_v3.sil              <- LIVE covenant source (F1 fix)
    bitphoque_compiled_real.json       <- compiled contract + genesis ctor args
  rust-examples\
    bitphoque_cli.rs                   <- CLI: all 9 subcommands, feature-complete
    bitphoque_genesis_establish.rs     <- genesis harness
    bitphoque_descriptor_v1.rs         <- parcel descriptor codec
    bitphoque_node_diag.rs             <- node health probe
    bitphoque_inspect_v3.rs            <- on-chain inspect (retry, pubkey-only)
    bitphoque_sale_claim_v3.rs         <- claim harness (3-field EngineFlags)
    (+ reject tests, sale harnesses, keygen, merge, split-transfer)
  web\
    wallet.html                        <- the wallet (v2, card-based UI)
    wallet_old.html                    <- pre-v2 backup (delete after testing)
    fix_price.ps1                      <- one-shot edit script (delete after testing)
    bitphoque_p2sh.js                  <- in-browser P2SH derivation
    bitphoque_sighash.js               <- sighash (keyed blake2b)
    bitphoque_sigscript.js             <- ABI sigscript builder
    bitphoque_tx.js                    <- tx builder + selftests
    bitphoque_descriptor.js            <- parcel descriptor codec
    bitphoque_qr.js                    <- QR SVG wrapper
    kaspa-logo.png                     <- Kaspa logo for powered-by banner
    phoque2.wav                        <- audio for lander button
    vendor/                            <- @scure/bip39, @scure/bip32, @noble/curves,
                                          @noble/hashes, qrcode-generator.mjs — all vendored
  docs\
    bitphoque_status.md                <- this file
    bitphoque_history.md               <- on-chain tx ledger + session history
    bitphoque_audit_2026-06-25.md      <- security audit
    bitphoque_swap_covenant_design.md  <- sale/swap design + auction future-build
    bitphoque_wallet_v2_design.md      <- wallet v2 design spec
    bitphoque_mainnet_checklist.md     <- pre-mainnet gate
    vps_handoff.md                     <- VPS setup + operational reference
    operating_baseline.md              <- shared portable baseline
```

Own git repo (branch `master`). Not yet pushed to GitHub.

---

## REFERENCE (adapted sources)

- `kcc20.sil` — token contract. `kcc20-minter.sil` — deliberately NOT used.
- `covenant_escrow.sil` / `covenant_mecenas.sil` — output-payment-inspection patterns.
- `kcc20_tests.rs` + `common.rs` — leader/delegate sigscript, local-test pattern.
- `crypto/txscript/src/lib.rs` + `script_builder.rs` — element cap + with_flags.
- `consensus/core/src/mass/mod.rs` — calc_storage_mass, ComputeBudget, script-units.
- `consensus/core/src/hashing/sighash.rs` — calc_schnorr_signature_hash.
- `consensus/core/src/hashing/hashers.rs` — TransactionSigningHash key.
- `silverscript-lang/src/compiler/mod.rs` — build_sig_script_for_covenant_decl.

---

## PARKED DESIGN — see `bitphoque_swap_covenant_design.md`

1. Gas-tank fee sizing (buyer 900k/claim, lister 700k/listing).
2. Wallet sends pups + dust (derive dust floor from KIP-9).
3. Marketing audience (Kaspa-native / meme-culture / self-erecting-thesis).
4. "Boques" = colloquial for phoques; denomination story.
5. Price-escalating emission = step bonding curve.
6. Wallet clarity (KAS vs phoque disambiguation, unit-confusion audit).
7. Key naming — legacy buyer1/2/3 rename sweep deferred.

---

## OPEN ITEMS

- [x] Onboarding, read side, write side — all proven live
- [x] All panels wired — send, split, list, partial-list, claim, merge
- [x] CLI feature-complete
- [x] VPS deployed — Hetzner CX33, Caddy TLS, `wss://node.outlawpolitics.org`
- [x] Site published to cPanel
- [x] Wallet v2 published (card-based UI, sompi→KAS fix)
- [ ] VPS IBD completion — verify `synced=true`
- [ ] Multi-entity testing (Entity1–4 cross-device send/claim/list)
- [ ] Parcel picker UX (dropdown from tracked parcels)
- [ ] `max(price, CLAIM_PAYMENT)` fix in `buildClaimTx`
- [ ] SMS Twilio node-monitor
- [ ] Original-token zero-pubkey owner quirk (observed, not blocking)
- [ ] Auction — separate future build (stateful/temporal)
- [ ] Mainnet: clear `bitphoque_mainnet_checklist.md`
  - NOTE: re-genesis produces a new `covenant_id`; sweep `3baed5a3…` in ALL JS + harnesses
- [ ] Make public on GitHub (clean repo name + README + secret-sweep)
- [ ] D: build-clone bin sweep (recopy all 14 canonical harnesses → clone, rebuild)
- [ ] Delete `C:\_RETIRED_kaspa-tn12` (file-handle release pending)
- [ ] Prune dead `import { decodeDescriptor }` at wallet.html line ~75
- [ ] Update shared panel banner ("Prototype — node-free…") — now false
- [ ] Clean up `wallet_old.html` + `fix_price.ps1` from `web\`
