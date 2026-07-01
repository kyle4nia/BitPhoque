# BitPhoque -- Status

_Read `operating_baseline.md` alongside this doc. BitPhoque lives in its own tree (`C:\bitphoque`)
and its own git repo -- separate from oracle-protocol. End-of-session checkpoint covers both trees._

_The on-chain tx ledger (every confirmed txid) and session-by-session history live in
`bitphoque_history.md`. Start here; reach for history only to look up a past fact._

---

## WHERE WE ARE (2026-06-29)

The web wallet is a **spending wallet**. Every layer of the write seam is proven against ground
truth and live on TN10.

- **Onboarding:** BIP-39 12-word, real entropy (`@scure/bip39`), AES-GCM encrypted persistence,
  HD key derivation (`m/44'/111111'/0'/0/0` -> secp256k1 x-only -> Kaspa bech32), cross-checked vs
  KasWare.
- **Read side:** `refreshBalance()` -- per-parcel P2SH derivation (template splice,
  `bitphoque_p2sh.js`) -> `getUtxosByAddresses` over JSON wRPC (`wss://node.outlawpolitics.org`) ->
  covenant filter -> reconcile + prune.
- **Write side (PROVEN LIVE):** sighash (`bitphoque_sighash.js`) -> schnorr sign (`@noble/curves`)
  -> ABI sigscript (`bitphoque_sigscript.js`) -> JSON submit via `wrpcCall("submitTransaction")`.
  **Live mined tx `4f3d791f`** -- wallet sent 1000 phoques, node accepted.
- **Panels wired:** send, split, list, partial-list, claim, merge -- all real build->sign->submit.
  Claim panel proven live (commit `70e9f96`): 3 txs (`d941a074`, `a8bc5081`, `77d595d8`).
  Merge panel proven live (commit `b04c041`).
- **CLI:** feature-complete -- import / balance / list / split / merge / partial-list / claim /
  send / receive. All proven on-chain.
- **Audit:** `bitphoque_audit_2026-06-25.md` -- no fund-loss or supply-break vector found.

### Wallet v2 -- PUBLISHED (2026-06-28)

Wallet v2 replaces the original `wallet.html`. Card-based UI, no protocol jargon.
Published at `https://bitphoque.org/wallet.html` (Hetzner VPS, Caddy static serving).

Commit `9406efb` covers the full session:
- Sompi->KAS: Buy panel price input now takes KAS, converts internally. No user-facing sompi.
- **Listing descriptor emit (Sell):** when a list tx confirms, wallet encodes a parcel descriptor
  with `forSale: true` + price. Uses the existing 118B descriptor codec -- no new format. Submit
  handler label is context-aware ("Share this listing code with buyers" vs "Give this code to
  the recipient").
- **Listing code auto-fill (Buy):** new "Listing code" field at top of Buy panel. Paste a
  descriptor -> `decodeListing()` decodes via `bitphoque_descriptor.js`, auto-fills seller key,
  txid, index, amount, price. Detects receive-vs-listing codes. Manual entry still works below.
- **KAS-for-fees collapsible:** home screen shows "Need KAS for fees?" dropdown with wallet
  address + faucet link. Shows even when node is unreachable (`kasBalance === null`).
- **Get Started on lander:** `index.html` gets a 6-step collapsible: create wallet, get KAS,
  receive, send, sell, buy. Same visual style as "Learn about this project."
- Onboarding "done" screen now tells new users to look for "Need KAS for fees?" on home screen.

Prior commits: `2c4f55c` (seal resize, welcome screen, QR, collapsibles, Receive panel
pubkey+QR, audio), `39b8b88` (trademark on seal, powered-by-Kaspa banner).

**Wallet v2 design spec** at `docs/bitphoque_wallet_v2_design.md`. Open design questions:
auto-claim vs manual, card aggregation, multi-parcel send.

### Cross-device testing + claim/send fixes (2026-06-28 evening)

Commit `95edb06`: max(price, CLAIM_PAYMENT) fix in `buildClaimTx`, mojibake cleanup, LAN node
detect (`192.168.0.118` -> Big Daddy). Commit `96fe14b`: auto-merge wiring in Send panel
(chain 2-at-a-time when no single parcel covers the send amount).

Proven live this session: Sell->Buy via listing code cross-device (Entity 1 -> Entity 3, 5 KAS
price), Send + implicit split (Entity 1 -> Entity 3, 351 from 850 parcel), LAN node routing
(phone at `192.168.0.118` -> Big Daddy `ws://192.168.0.118:18210`).

### Bug triage + fixes (2026-06-28 late evening, sig #001)

Consolidated findings from two Claude Pro accounts (desktop + browser) that had drifted.
Loaded session findings, console evidence, and VPS handoff notes. Diagnosed four bugs:

1. **Send-to-self output 0 vanishes -- ROOT CAUSE FOUND, FIX APPLIED, PROVEN LIVE (sig #002).**
   Traced through `buildSendTx` -> `buildOwnerSignedTx` -> `deriveScriptHashHex` ->
   `deriveRedeemScript`. All derivation code is correct. The asymmetry: output 0 uses
   `recipientPubkey` (from the "To" input field) while output 1 uses `inputOwner` (from the
   wallet key). For send-to-self, these should match but came from different sources. Console
   diagnostic confirmed derivation is deterministic -- the key pasted into "To" didn't match
   `myKey.pubHex`. Fix: self-send normalization in `doSend()` -- if `xonly(to) === xonly(myKey.pubHex)`,
   force `to` to `xonly(myKey.pubHex)`.
2. **Restore flow missing passphrase save -- NOT A BUG.** `prepDone()` hides the save box when
   `window.isSecureContext === false`. Entity 3 (phone) accessed via HTTP over LAN IP
   (`192.168.0.118`), which is not a secure context, so `crypto.subtle` is unavailable. Resolves
   when phone uses `https://bitphoque.org` (HTTPS = secure context).
3. **Refresh button appears broken -- NOT A BUG.** `refreshAll()` is correctly wired
   (`onclick="refreshAll()"` in plain `<script>` block, global scope). No `console.log` on the
   success path -- "no console output" is expected when it works. Likely appeared broken due to
   phantom parcels from bug #1.
4. **Listing descriptor below the fold -- FIXED.** Added `out.scrollIntoView({behavior:'smooth',
   block:'end'})` after tx confirmation text in `submitPendingTx()`.

Also established: session signature system for cross-account status doc synchronization.

### Config extraction + deployment structure (2026-06-29, sig #005)

Commit `52d6f24`: new `bitphoque_config.js` module (NETWORK, COVENANT_ID_HEX,
COVENANT_ID_SHORT, WRPC_URL_PUBLIC). All scattered hardcoded values extracted:
`bitphoque_tx.js` imports COVENANT_ID_HEX, `bitphoque_descriptor.js` imports
COVENANT_ID_HEX, `bitphoque_p2sh.js` imports NETWORK, `wallet.html` 9 'kaspatest'
literals reduced to 1 (line 61 config point) + 1 COVENANT_ID (line 649). To flip
networks: change `bitphoque_config.js` + 2 wallet.html lines.

Deployment structure: `testnet.bitphoque.org` (Caddy site block, `/var/www/bitphoque-testnet/`)
+ `bitphoque.org` (becomes mainnet at launch). Both HTTPS, both live. Same code, one config
difference at flip time.

VPS node: `--addpeer=47.135.159.199:16211` added to kaspad service (Big Daddy P2P feed,
port-forwarded). Sync rate jumped from 198 blocks/10s with 0 UTXO-validated to 300+/10s
with 100+ validated. Volume resized 150G -> 200G (66G free at 26% UTXO validation).

### Parcel registry -- LIVE (2026-06-30, sig #007)

Bulletin board extended into parcel registry. Server: `POST /api/parcels` registers
output parcels (deduped by outpoint), `GET /api/parcels?owner=<pubkey>` returns all
known parcels for an owner. Wallet: `discoverParcels()` queries registry on every
refresh, validates each candidate on-chain before adding. `registerParcels()` POSTs
output parcels on every tx confirm. `refreshBalance()` seeds registry with all confirmed
parcels on every refresh. Fresh wallet restored from seed phrase discovers held phoques
automatically -- no descriptors needed. Brute-force scan (1-42,069 amounts, 500/batch)
used for one-time bootstrap. Commit `ba57407`.

### VPS + PUBLIC DEPLOY -- DONE (2026-06-28)

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

**Static hosting** also on VPS via Caddy: `bitphoque.org` + `www.bitphoque.org` ->
`/var/www/bitphoque`; `testnet.bitphoque.org` -> `/var/www/bitphoque-testnet/`. No separate web host needed.

**Deploy workflow:** edit locally -> move to `C:\bitphoque\web\` ->
`cd /d C:\bitphoque\web && scp <file> root@89.167.32.239:/var/www/bitphoque/`

**SSH key auth configured** -- no password prompt for ssh/scp to VPS.

SSH: `ssh root@89.167.32.239`. Full operational details in `vps_handoff.md`.

**Node sync status (2026-06-29):** UTXO validation ongoing, 71% volume (56G free), ~200 blocks/10s via Big Daddy addpeer.
+8G swapfile (12G total swap), `TimeoutStopSec=300`, `LimitNOFILE=65536`. No OOM, no stalls.
Volume resized 150G -> 200G (66G free). CX42 (16GB, 16.40 EUR/mo) is the upgrade if bandaid fails.

**wallet.html auto-detect:** `location.hostname === "127.0.0.1"` -> Big Daddy
(`ws://127.0.0.1:18210`); `192.168.0.118` -> LAN Big Daddy (`ws://192.168.0.118:18210`);
else -> VPS (`wss://node.outlawpositics.org`). wRPC timeout bumped to 30s (300M UTXO set
needs headroom).

---

## What this is (one sentence)

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 -- a practice run for
mainnet -- built on silverscript covenants (`binding=cov` N-to-M plumbing), deliberately NOT the
indexer-based KRC-20 path. Phonetic play on "out of f**ks to give," seal mascot, fixed supply, no
trusted coordinator. Site: https://bitphoque.org/

---

## DESIGN -- SETTLED

- **Architecture:** per-UTXO balance (real token, not a single-UTXO ledger). Conservation-enforced.
- **Ownership:** per-UTXO signature, `IDENTIFIER_PUBKEY=0x00` (plain checkSig).
- **Supply:** FIXED at genesis, NO minter. 42,069 phoques. `genesisIsMinter=false`.
- **Subdivision -- BitPups.** 1 phoque = 100,000,000 BitPups (8 dp). TN10 minted as whole units
  (needs re-genesis for mainnet sub-unit headroom).
- **Sale model (v3):** per-UTXO `forSale` + `price`. Listing = owner-signed. Claim =
  payment-authorized. F1 fix: strict-increasing payment-output indices.
- **Wallet custody:** non-custodial, BIP-39 12-word, key in-browser, never on server.
- **Receive:** off-chain signed parcel descriptor (118B, base58check) + on-chain confirm.
  Descriptor = claim, chain = truth.
- **KAS handling:** "gas-tank, not vault." Fee floors: 700k list/split, 900k claim, 1.3M merge.

---

## THE CONTRACTS

### Original token -- `covenants/bitphoque.sil` (DEPLOYED)
Copied from `kcc20.sil`. State: ownerIdentifier + identifierType + amount + isMinter.
ABI: `__leader_transfer` + `__delegate_transfer`.

### Sale token v3 -- `covenants/bitphoque_sale_v3.sil` (LIVE)
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
2. Covenant input: 111,261 script units -> ComputeBudget(11). Plus ~100k for plain P2PK -> also ComputeBudget(11).
3. KIP-9 dust penalty: pay non-dust amounts (>=100M sompi for claims).
4. Fee floors: genesis 200k; transfer 400k; list/split 700k; claim 900k; merge 1.3M; 2x2 claim 1.5M.
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
    lockTime: 0, subnetworkId: "00...00" (40 hex), gas: 0, payload: "", mass: 0
  },
  allowOrphan: false
})
```

### Node quirks
- Address-index lags tx-index by a few seconds. Mitigation: 3 retries, 3s apart.
- No by-outpoint UTXO query -- `get_utxos_by_addresses` only, filter client-side.

---

## TN10 GROUND FACTS

- **Local node (Big Daddy):** `127.0.0.1:16210` gRPC, `18210` JSON wRPC. kaspad 2.0.1.
  Launch: `kaspad.exe v2.0.1 --testnet --netsuffix=10 --appdir=C:\kaspa-testnet10\datadir
  --utxoindex --disable-upnp --rpclisten=0.0.0.0:16210 --rpclisten-borsh=0.0.0.0:17210
  --rpclisten-json=0.0.0.0:18210`.
- **VPS node:** Hetzner CX33, `89.167.32.239`. `wss://node.outlawpolitics.org`.
  SSH: `ssh root@89.167.32.239`. Details in `vps_handoff.md`.
- **Build clone:** `D:\Tocatta_Projects\kaspa-tn12\silverscript`.
  **Recopy canonical->clone before running any harness from D:**.
- **NODE addr rule:** every `.rs` `const NODE` = `127.0.0.1:16210`. No machine IPs.

### Test entities (PUBLIC keys only)

| Entity | Label | Address | Pubkey (hex) | TKAS |
|--------|-------|---------|-------------|------|
| 1 | phoque (webapp) | `kaspatest:qzcpdfrm...` | `1602d48f...` | ~2,999 |
| 2 | kasware | same as Entity 1 (same seed) | same | ~2,999 |
| 3 | phonePhoques (phone) | `kaspatest:qzhl95s2...` | `15fe5a41...` | 1,000 |
| 4 | tabletphoques (tablet) | `kaspatest:qpx2a6yx...` | `0995dd10...` | 1,000 |

**Legacy key labels** (in harness source + `.key` files): buyer1 `8c02c20a...`, buyer2 `5d338aa5...`,
buyer3 `267496ce...`.

- **Faucet:** min 1000 TKAS/request, 24h cap 100k.

### Genesis vs parcel P2SH
Genesis = route-1 (`blake2b(entire compiled script)` -> P2SH). Parcels = route-2 (template
splice -> `blake2b` -> P2SH). Different scripts, different addresses. Wallet derives route-2 only.

---

## KEY HISTORY (detail in `bitphoque_history.md`)

- **Toolchain split (2026-06-26):** 2-field vs 3-field `EngineFlags` mismatch. Fixed via
  `[patch]` block in `D:\...\silverscript\Cargo.toml` redirecting all kaspa crates to local
  3-field `rusty-kaspa`. Green-path proven: CLI send -> web Receive -> parcel confirmed.
- **Node addr sweep (2026-06-27):** all 14 `.rs` harnesses repointed from stale machine IPs
  to `127.0.0.1:16210`. Commit `e3f6628`.
- **Wallet v2 + listing descriptors (2026-06-28):** wallet_v2.html published as wallet.html.
  Listing descriptor emit on Sell, auto-fill on Buy, KAS collapsible, Get Started on lander.
  VPS IBD completed. Commit `9406efb`.
- **bitphoque.org hosting (2026-06-28):** DNS (Namecheap A records) -> Hetzner. Caddy site
  block for static serving. SSH key auth. Deploy via scp. wallet.html auto-detect node URL +
  30s timeout. Operating baseline updated (3 additions). VPS node restarted after post-import
  stall.
- **VPS upgrade (2026-06-27):** CX22 -> CX33 after kaspad OOM during IBD.
- **Claim harness EngineFlags fix (2026-06-27):** added `zk_hardening_enabled: true`.
- **Cross-device testing (2026-06-28 evening):** Sell->Buy proven cross-device. max(price,floor)
  fix. Auto-merge in Send. LAN node detect. Send-to-self bug found (output 0 vanishes).
  Commits `95edb06`, `96fe14b`.
- **Bug triage + fixes (2026-06-28 late, sig #001):** Consolidated two drifted accounts.
  Send-to-self root cause: key source mismatch, not derivation bug. Self-send normalization
  fix + listing descriptor scroll fix applied. Restore passphrase and refresh button
  investigated -- both not bugs. Session signature system established.
- **Parcel registry (2026-06-30, sig #007):** bulletin.py extended with /api/parcels.
  Wallet discovers parcels from registry on open, seeds on refresh, registers on tx
  confirm. Brute-force scan bootstrapped 3 parcels for Entity 1. Commit `ba57407`.
- **Config extraction + deployment structure (2026-06-29, sig #005):** `bitphoque_config.js`
  created, all modules + wallet.html wired to single config surface. `testnet.bitphoque.org`
  deployed (Caddy + TLS). VPS node addpeer to Big Daddy, volume 150G->200G. Commit `52d6f24`.
- **Send-to-self proven + deploy (2026-06-29, sig #002):** Self-send live-tested (107+42 both
  visible). wallet.html redeployed to VPS. Deferred cleanup resolved. Three-part session
  record added to operating baseline (both trees). Commit `03d6bdf`.

---

## FILE TREE (`C:\bitphoque`)

```
C:\bitphoque\
  .gitignore                           <- *.key + wallet.json
  bitphoque_owner.key                  <- original/funding signer (GITIGNORED)
  bitphoque_buyer1.key                 <- buyer1 8c02c20a... (GITIGNORED)
  bitphoque_buyer2.key                 <- buyer2 5d338aa5... (GITIGNORED)
  bitphoque_buyer3.key                 <- buyer3 267496ce... (GITIGNORED)
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
    bitphoque_config.js                <- network config (testnet/mainnet flip)
    bitphoque_p2sh.js                  <- in-browser P2SH derivation
    bitphoque_sighash.js               <- sighash (keyed blake2b)
    bitphoque_sigscript.js             <- ABI sigscript builder
    bitphoque_tx.js                    <- tx builder + selftests
    bitphoque_descriptor.js            <- parcel descriptor codec
    bitphoque_qr.js                    <- QR SVG wrapper
    kaspa-logo.png                     <- Kaspa logo for powered-by banner
    phoque2.wav                        <- audio for lander button
    vendor/                            <- @scure/bip39, @scure/bip32, @noble/curves,
                                          @noble/hashes, qrcode-generator.mjs -- all vendored
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

- `kcc20.sil` -- token contract. `kcc20-minter.sil` -- deliberately NOT used.
- `covenant_escrow.sil` / `covenant_mecenas.sil` -- output-payment-inspection patterns.
- `kcc20_tests.rs` + `common.rs` -- leader/delegate sigscript, local-test pattern.
- `crypto/txscript/src/lib.rs` + `script_builder.rs` -- element cap + with_flags.
- `consensus/core/src/mass/mod.rs` -- calc_storage_mass, ComputeBudget, script-units.
- `consensus/core/src/hashing/sighash.rs` -- calc_schnorr_signature_hash.
- `consensus/core/src/hashing/hashers.rs` -- TransactionSigningHash key.
- `silverscript-lang/src/compiler/mod.rs` -- build_sig_script_for_covenant_decl.

---

## PARKED DESIGN -- see `bitphoque_swap_covenant_design.md`

1. Gas-tank fee sizing (buyer 900k/claim, lister 700k/listing).
2. Wallet sends pups + dust (derive dust floor from KIP-9).
3. Marketing audience (Kaspa-native / meme-culture / self-erecting-thesis).
4. "Boques" = colloquial for phoques; denomination story.
5. Price-escalating emission = step bonding curve.
6. Wallet clarity (KAS vs phoque disambiguation, unit-confusion audit).
7. Key naming -- legacy buyer1/2/3 rename sweep deferred.

---

## OPEN ITEMS

- [x] Onboarding, read side, write side -- all proven live
- [x] All panels wired -- send, split, list, partial-list, claim, merge
- [x] CLI feature-complete
- [x] VPS deployed -- Hetzner CX33, Caddy TLS, `wss://node.outlawpolitics.org`
- [x] VPS IBD complete -- node synced, 300M UTXOs
- [x] Site published -- `bitphoque.org` on Hetzner (Caddy static + TLS)
- [x] Wallet v2 published (card-based UI, listing descriptors, KAS collapsible, Get Started)
- [x] `max(price, CLAIM_PAYMENT)` fix in `buildClaimTx` (commit `95edb06`)
- [x] Auto-merge wiring in Send panel (commit `96fe14b`)
- [x] Hetzner swap bandaid applied (+8G swapfile, 12G total, systemd tweaks)
- [x] Send-to-self root cause identified + normalization fix applied (sig #001)
- [x] Listing descriptor scroll fix applied (sig #001)
- [x] Restore passphrase save investigated -- secure-context gate, not a bug (sig #001)
- [x] Refresh button investigated -- not broken, no console.log on success path (sig #001)
- [x] Session signature system established (sig #001)
- [x] **Live-test send-to-self fix** -- proven live, both outputs visible (sig #002)
- [x] **Deploy wallet.html to VPS** -- deployed (sig #002)
- [x] Update hardcoded `127.0.0.1:8080` links -- already clean (sig #002)
- [x] Clean up `wallet_old.html` from `web\` -- already gone (sig #002)
- [x] Clean up `fix_price.ps1` from `web\` -- already gone (sig #002)
### Deployment-critical (testnet trial)
- [x] Sent receipts persistence -- localStorage + Sent receipts panel, descriptors saved on encode (commit `f1c6bdc`, sig #006)
- [x] Owner filtering -- isOwn() guard on all parcel selection, multi-key wallet fix (commit `f1c6bdc`, sig #006)
- [x] Parcel registry live -- discover on open, seed on refresh, register on confirm (commit `ba57407`, sig #007)

### Deployment-critical (testnet trial)
- [x] Hetzner node: sync confirmed complete (sig #008)
- [x] Multi-entity testing: OG->phone proven, OG->tablet diagnosed (descriptor data-entry issue, not code bug) (sig #008)

### Not needed for deployment
- [ ] cPanel: evaluate whether still needed (outlawpolitics.org has other content?)
- [ ] SMS Twilio node-monitor
- [ ] Auction -- separate future build (stateful/temporal)
- [x] Faucet URL updated to faucet-tn10.kaspanet.io (index.html + wallet.html, sig #009)
- [x] Sold-listing 1-week server-side expiry (bulletin.py + wallet.html, deployed, sig #009)
- [x] Contract confidence review: v3+v4+audit, high confidence on covenant logic (sig #009)
- [ ] Mainnet node IBD (kaspad-mainnet.service on VPS, 38% headers at sig #009 — monitor)
- [ ] v4 downstream code sync (sigscript_v4 done; descriptor OK; compiled_real/CLI/harnesses/wallet BLOCKED on V1 compile)
- [ ] Mainnet: clear `bitphoque_mainnet_checklist.md`
  - NOTE: re-genesis produces a new `covenant_id`; sweep `3baed5a3...` in ALL JS + harnesses
- [ ] Make public on GitHub (clean repo name + README + secret-sweep)
  - Minter branch already stripped in v4 source (`bitphoque_sale_v4.sil`). TN10 continues on v3 contract as-is; mainnet uses v4. Publish v4 source (no minter, no fork risk).
- [ ] D: build-clone bin sweep (recopy all 14 canonical harnesses -> clone, rebuild)
- [ ] Delete `C:\_RETIRED_kaspa-tn12` (file-handle release pending)

---

## SESSION SIGNATURES

_Append-only. Newest first. Format: seq | datetime TZ | agent | trees | scope | head._
---sig #009 | 2026-07-01 ~17:15 EST | desktop | trees: bitphoque | scope: housekeeping (faucet URL, sold-listing 1-week expiry), contract confidence review (v3+v4+audit), mainnet node spinup, v4 downstream sync start | head: 957c69d

OPENING ASSESSMENT: Two housekeeping chores queued (faucet URL update, sold-listing expiry), then mainnet prep. Both deployment-critical testnet items were resolved in sig #008. Live wallet functional, VPS healthy. Plan: knock out housekeeping, then focus on mainnet readiness — contract review, VPS mainnet node, v4 code sync.

CLOSING: Both housekeeping items shipped and deployed to VPS. Faucet URL updated to faucet-tn10.kaspanet.io across index.html + wallet.html (3 references). Sold-listing expiry redesigned from client-side in-memory guess (_soldAt, 5-min window) to server-authoritative 1-week retention: bulletin.py gained POST /api/listings/sold endpoint + prune_sold_listings() auto-cleanup on GET, wallet.html now reports sold listings to server on discovery (first-browser-to-notice pattern, idempotent). Full buy/sell/send/receive cycle confirmed working post-deploy.

Contract confidence review: read v3 source line-by-line, independently confirmed all four core invariants (conservation, no-remint, no-relist-after-claim, no-double-claim). Found the same amount>0 gap the sig #004 session caught — confirmed closed in v4's checkPositiveAmounts(). Read the June 25 audit (history-blind, all findings reconciled or already resolved). Read v4 source, diffed against v3 — strict hardening, no new risk. Read mainnet checklist — comprehensive, correctly identifies silverscript V1 as the blocking dependency. Confidence: HIGH on covenant logic, LOW (correctly) on mainnet readiness (V1 compiler still unreleased, confirmed via search — repo README still says "experimental"). Toccata consensus hard fork activated on mainnet June 30 — separate from the compiler blocker.

Mainnet node spun up on VPS: kaspad-mainnet.service (systemd, same v2.0.1 binary as TN10, no --archival, --utxoindex, loopback RPC only). Datadir auto-landed on the 200G block-storage volume via existing /root/.rusty-kaspa symlink. IBD healthy at 38% headers / 1.7G at session end. Toccata activation boundary correctly recognized during sync ([TOCC] log line). Mainnet default ports (16110/16111/17110/18110) confirmed free before start. VPS disk analysis: TN10 already pruned (no --archival flag), 104G is mostly the 300M-UTXO index, not retainable history. retention-period-days confirmed: unset = minimal pruning window (~30h at 10BPS), the flag only allows MORE retention not less. Mainnet UTXO set expected smaller than TN10's synthetic 300M.

v4 downstream sync started: bitphoque_sigscript_v4.js delivered (4-field/49-byte state layout, identifierType + isMinter dropped, structural self-test passing in Node.js, full buildSigScript leader+delegate paths execute clean). Filed as _v4 side-by-side — does NOT overwrite v3, live testnet wallet unaffected. bitphoque_descriptor.js confirmed already v4-compatible (transport format never carried the dropped fields). bitphoque_compiled_real.json identified as compiler output (ctor args baked into bytecode) — gets completely replaced by V1 compile, no useful pre-build possible. Rust CLI + harnesses same story — blocked on V1, documented.

Stray zero-byte `ssh` file in repo root deleted (mistyped redirect artifact). Deferred .ps1 scripts in Downloads cleaned during session.

DRIFT: No major misdiagnosis. The sold-listing expiry required a deeper redesign than expected — the existing _soldAt was purely client-side in-memory with a 5-minute window, which meant "1 week from sale" was architecturally impossible without moving the timestamp to the server. Flagged this proactively before writing any code, user confirmed server-side approach. Contract review went smoothly — every gap I found independently was already caught by either the June 25 audit or the sig #004 session, which is the strongest signal the review process can produce (three independent passes converging). The compiled_real.json turned out to be the original KCC20 contract, not v3 — state_layout {start:1, len:46} doesn't match v3's forSale/price fields. The actual v3 compiled output either lives elsewhere or was used directly during genesis. Not a problem for mainnet (v4 gets a fresh compile), but worth noting. The F1 strict-increasing comment reframe in v4 (from "testnet shortcut" to "production design") was flagged as a documentation regression — user confirmed it was a deliberate decision recorded in the mainnet checklist, not an accidental slip.



---sig #007 | 2026-06-30 ~09:00 EST | desktop | trees: bitphoque | scope: parcel registry (bulletin.py + wallet discover/register/seed), bootstrap scan | head: ba57407

OPENING ASSESSMENT: VPS node synced. Wallet shows KAS balance on open but not phoque state. Suspected missing refresh-on-open call. Plan: wire phoque refresh on wallet open.

CLOSING: Root cause was architectural: KAS lives at one P2PK address, but each phoque parcel has a unique P2SH (owner+amount+forSale+price). No single address to scan. Built parcel registry: bulletin.py extended with /api/parcels (POST to register, GET by owner). Wallet wired with discoverParcels() on refresh, registerParcels() on tx confirm, and seed-on-refresh in refreshBalance(). Bootstrap required brute-force scan (42,069 addresses, 500/batch) since no wallet had parcels in localStorage to seed. 3 parcels found for Entity 1 (35+1+3000). Fresh wallet restore from seed confirmed working -- parcels appear automatically.

DRIFT: Initial assumption (missing refresh-on-open) was wrong -- refreshAll() already fires on unlock. Real issue was that refreshBalance() is a validator, not a scanner. Pivoted from simple wiring fix to full registry feature. Bulletin board server endpoint worked immediately; wallet-side wiring landed clean but seed POST was silent (no console logging), requiring manual diagnostics to confirm. LocalStorage was empty in all browsers, so CLI wallet.json and brute-force scan were the only bootstrap paths.

---sig #006 | 2026-06-29 ~16:30 EST | desktop | trees: bitphoque | scope: cache busting (CACHE_V), sent receipts persistence, owner filtering for multi-key wallets | head: f1c6bdc

OPENING ASSESSMENT: Cache busting was flagged as next priority (sig #005). VPS node still syncing. No known bugs. Plan: add cache busting to all dynamic imports.

CLOSING: Cache busting done (CACHE_V constant + 18 imports). Discovered and fixed two bugs during testing: (1) lost descriptor problem (send window closes = descriptor gone forever) -- fixed with sentReceipts localStorage persistence + Sent receipts panel; (2) multi-key wallet contamination (parcels from Entity 1 key visible/selectable when unlocked with owner key) -- fixed with isOwn() filter on all parcel selection points. Both deployed to bitphoque.org + testnet.bitphoque.org. VPS node healthy, 71% volume, syncing.

DRIFT: Cache busting was straightforward. Bulk of session was unplanned bug work from live testing. Sent receipts emerged from lost-descriptor question. Owner filtering emerged from send failure tracing to mixed-key parcels in shared localStorage. Double-run of .ps1 script caused duplication requiring dedup -- lesson: edit scripts whose target strings survive the first run will double-insert on re-run. What appeared to be identical receive codes were actually different -- they share a long base58 prefix (first 33 bytes = version + covenant_id are constant).

---sig #002 | 2026-06-29 00:00 EST | desktop | trees: bitphoque, oracle-protocol

OPENING ASSESSMENT: Believed send-to-self fix was applied but untested (top blocker), VPS wallet.html was stale, VPS node syncing but unconfirmed, two deferred deletes pending (wallet_old.html, fix_price.ps1), hardcoded 127.0.0.1:8080 links needed updating. Plan: live-test self-send, deploy to VPS, check node, clean up deferred items.

CLOSING: Send-to-self proven live (107+42 both visible). wallet.html deployed to VPS via scp. VPS node healthy at 16% IBD (~7-8h remaining). Three-part session record added to operating baseline (both trees committed). Stale dated file session_findings_2026-06-28.md deleted. Deferred cleanup items all resolved (wallet_old.html already gone, fix_price.ps1 already gone, hardcoded links already clean).

DRIFT: Opening assumed deferred deletes and link cleanup were actual work items -- all three were already resolved before this session. The real work was just the self-send test and VPS deploy. No misdiagnosis on the send-to-self fix itself -- it worked as described in sig #001. IBD is a wait, not a task. | head: 03d6bdf

---sig #001 | 2026-06-28 23:02 EST | browser | trees: bitphoque | scope: consolidated two drifted accounts, diagnosed send-to-self bug (key source mismatch not derivation), self-send normalization fix, listing descriptor scroll fix, restore/refresh investigated (not bugs), session signature system established | head: 9e9640f
---sig #003 | 2026-06-29 ~11:00 EST | desktop | trees: bitphoque | scope: VPS node check (91% IBD), v3 genesis owner field test (not reproducible, item closed), bulletin board (Python service + Caddy proxy + wallet integration), lazy phoque message | head: 1fb7837

OPENING ASSESSMENT: VPS node was at 16% IBD (sig #002). Zero-pubkey quirk was open with no detail. Bulletin board was an open item. Plan: check node, investigate quirk, build bulletin.

CLOSING: VPS node at 91% IBD, healthy. Zero-pubkey quirk closed — throwaway v3 genesis confirmed owner baked correctly (tx 0c81fda9, cleaned up). Bulletin board live: Python service on VPS (systemd), Caddy /api/* proxy, wallet Sell POSTs listing on confirm, Buy has Browse button with tappable cards, absolute URLs work from any node. Lazy phoque message for stale listings.

DRIFT: Zero-pubkey quirk took less time than expected — single genesis+inspect cycle settled it. Bulletin board went smoothly — no compile issues, no CORS problems. The only surprise was relative vs absolute fetch URLs (local wallet couldn't reach /api/listings), fixed in-session.
---sig #005 | 2026-06-29 ~14:00 EST | desktop | trees: bitphoque | scope: config extraction (bitphoque_config.js), deployment structure (testnet.bitphoque.org), VPS addpeer + volume resize | head: 52d6f24
OPENING ASSESSMENT: VPS node syncing slowly (91% header, 0 UTXO-validated). Config values hardcoded across 4 JS files + wallet.html. No testnet/mainnet deployment separation. Volume at 83% (25G free). Plan: speed up sync via addpeer, extract config, set up deployment structure.
CLOSING: VPS syncing at 300+ blocks/10s with 100+ UTXO-validated via Big Daddy addpeer. Config extraction complete (1 config file, 3 modules + wallet wired). testnet.bitphoque.org live with TLS. Volume resized 150G->200G. Cache busting flagged as next priority.
DRIFT: VPS UTXO validation restart (5% then climbing) was unexpected -- restart resets validation counter even though headers were 91%. Disk math showed volume would run out (~3G margin) so volume resize was unplanned but necessary. Config extraction and deployment structure went smoothly with no surprises.
---sig #004 | 2026-06-29 ~12:00 EST | desktop | trees: bitphoque | scope: contract mainnet review — v4 covenant source, mainnet checklist expansion | head: 48b7674
OPENING ASSESSMENT: Contract had four open review items flagged in the status doc (F1 strict-increasing vs per-output accounting, caps 3/3 sizing, price=0 wallet guard, minter branch strip). Mainnet checklist existed but didn't detail specific source changes. Plan: resolve all four, draft v4 covenant, update checklist.
CLOSING: All four items resolved. Two additional issues found and fixed (amount>0 guard, identifierType strip). bitphoque_sale_v4.sil delivered to covenants\. Mainnet checklist expanded with items 5/7/8 and launch sequence updated to 11 steps. Status doc contract review item checked off. Everything remaining is blocked on silverscript V1 release.
DRIFT: No misdiagnosis. The four flagged items were real and resolved as expected. Two bonus items emerged from reading the source (zero-amount outputs, dead identifierType field) — neither was tracked before this session. No surprises, no blockers hit.
Session #008 — 2026-06-30
Completed:

Hetzner node sync confirmed ✓ (last deployment-critical item)
Multi-entity cross-device testing (OG → phone ✓, OG → tablet — diagnosed descriptor owner mismatch, confirmed data-entry issue not code bug)
QR scan button on Send panel — BarcodeDetector API, zero dependencies, camera icon next to To field, auto-fills pubkey on scan
QR UX refinement — smaller button (width:auto fix for .btn width:100%), persistent "✓ Key scanned" confirmation with key display (stays until Send or close)
Contacts / address book — localStorage-backed, tappable name chips in Send panel, "+ Save contact" flow, inline manage/delete UI
Auto-merge observed working cleanly in live cross-device testing
Both deployment-critical items now resolved

Found (non-critical):

Wrong-device receive: entering a descriptor on a device whose pubkey doesn't match prints "received" but balance stays 0. No on-chain harm (UTXO stays at real owner's P2SH). Fix: compare descriptor.ownerPubkey vs xonly(myKey.pubHex) in doReceive(), reject on mismatch.

Open items carried forward:

Wrong-device receive reject message (UX polish)
cPanel evaluation, SMS/Twilio monitor, auction, mainnet checklist, GitHub publication
Contract mainnet review (F1 accounting, caps sizing, price=0 guard, minter branch strip)