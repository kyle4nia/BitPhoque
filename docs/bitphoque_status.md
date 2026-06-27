# BitPhoque — Status

_Read `operating_baseline.md` alongside this doc. BitPhoque lives in its own tree (`C:\bitphoque`)
and its own git repo — separate from oracle-protocol. End-of-session checkpoint covers both trees._

_The on-chain tx ledger (every confirmed txid) and session-by-session history live in
`bitphoque_history.md`. Start here; reach for history only to look up a past fact._

---

## WHERE WE ARE (2026-06-27)

The web wallet is a **spending wallet**. Every layer of the write seam is proven against ground
truth and live on TN10:

- **Onboarding:** BIP-39 12-word, real entropy (`@scure/bip39`), AES-GCM encrypted persistence,
  HD key derivation (`m/44'/111111'/0'/0/0` → secp256k1 x-only → Kaspa bech32), cross-checked vs
  KasWare.
- **Read side:** `refreshBalance()` wired — per-parcel P2SH derivation (template splice,
  `bitphoque_p2sh.js`) → `getUtxosByAddresses` over JSON wRPC (`wss://node.outlawpolitics.org`) →
  covenant filter → reconcile + prune.
- **Write side (PROVEN LIVE):** sighash (`bitphoque_sighash.js`, keyed blake2b, validated vs Rust
  test vectors) → schnorr sign (`@noble/curves`) → ABI sigscript (`bitphoque_sigscript.js`,
  transposed State[], validated vs CLI hex dump) → JSON submit via `wrpcCall("submitTransaction")`.
  **Live mined tx `4f3d791f`** — wallet sent 1000 phoques, node accepted.
- **Panels wired:** send, split, list, partial-list, claim — all real build→sign→submit.
  Claim panel wired + proven live (commit `70e9f96`): auto-funding-UTXO picker, 2-input
  payment-authorized build, three live txs (`d941a074`, `a8bc5081`, `77d595d8`).
  Merge panel wired + proven live (commit `b04c041`).
- **CLI:** feature-complete — import / balance / list / split / merge / partial-list / claim / send /
  receive. All proven on-chain.
- **Audit:** `bitphoque_audit_2026-06-25.md` — history-blind source reconstruction, no fund-loss
  or supply-break vector found. R3 (minter branch annotated) and R5 (genesis node const repointed)
  closed. R1 (F-2 genesis derivation) retired by design (genesis uses route-1 whole-compiled-script,
  parcels use route-2 splice — intentionally different, see below).

### VPS + PUBLIC DEPLOY — DONE (2026-06-27)

**Hetzner VPS (CX22, Helsinki, Ubuntu 24.04)** provisioned as the public-facing TN10 node.

- **kaspad v2.0.1** installed, running as systemd service (`/etc/systemd/system/kaspad.service`).
  Flags: `--testnet --netsuffix=10 --utxoindex --rpclisten-json=0.0.0.0:18210`.
- **Caddy reverse proxy** auto-provisions Let's Encrypt TLS.
  `wss://node.outlawpolitics.org` (443) → `ws://127.0.0.1:18210` (local kaspad).
- **DNS:** `node.outlawpolitics.org` A record → `89.167.32.239` (via cPanel Zone Editor).
- **wallet.html WRPC_URL** changed from `ws://127.0.0.1:18210` to `wss://node.outlawpolitics.org`.
- **Full site published to cPanel** — `public_html/BitPhoque/`: index.html, wallet.html,
  favicon.ico, bitphoque-nowords.jpeg, phoque3.wav, all JS modules, vendor/.
- **IBD in progress** at session end (downloading pruning-point chain segment). Wallet calls
  will fail until `synced=true`. Check: `ssh root@89.167.32.239` then
  `journalctl -u kaspad --no-pager -n 20`.

Architecture:
```
  Browser (anywhere)
    |  wss://node.outlawpolitics.org (port 443, TLS)
    v
  Caddy (VPS, TLS termination, Let's Encrypt auto-renew)
    |  ws://127.0.0.1:18210 (local, plaintext)
    v
  kaspad (VPS, TN10, --utxoindex)
```
Wallet static files served by cPanel (outlawpolitics.org/BitPhoque/).
Node RPC served by Hetzner VPS. Completely separate.

Full VPS operational details in `vps_handoff.md`.

### Claim — PANEL WIRED + PROVEN LIVE (2026-06-27)

`buildClaimTx` added to `bitphoque_tx.js` (commit `f107c80`). 2-input, payment-authorized
claim — the structurally different tx type the wallet hadn't covered:

- **Input 0:** for-sale covenant UTXO, UNSIGNED (payment-authorized). Sigscript = leader ABI
  with dummy 65-zero sig + witness byte `[1]` pointing at the payment output. Redeem push
  (2793B). computeBudget 24.
- **Input 1:** buyer's plain P2PK funding UTXO, buyer-SIGNED at index 1. computeBudget 11.
- **Output 0:** token → buyer (covenant-bound, `authorizingInput: 0`, full parcel value,
  forSale=false).
- **Output 1:** payment → seller P2PK (100M sompi = 1 TKAS, KIP-9-safe; must be ≥ price).
- **Output 2:** change → buyer P2PK.
- **Fee:** 900k (consensus #9, 2-input claim floor 857k).

**Sighash crosscheck:** `claimSelfTest()` rebuilds the exact claim tx from a
`bitphoque_sale_claim_v3` dry-run (seller=buyer1 `8c02c20a…`, claimer=buyer3 `267496ce…`,
3000 phoques, price 500000, cov UTXO `8dff6df4…:0`, funding `40cbf865…:5`) and asserts
input-1 sighash = `de52d220051a7bc78426de553835b0ccd9aedf70a20341f721f423be70c1bae1`.
**PASS** — byte-identical to the Rust engine. Payment/change math verified. Sigscript
2922 bytes. The harness also ran LOCAL VM VERIFY: PASS on the covenant input.

**New helpers:** `makeP2pk(pubkeyHex)` builds a plain P2PK scriptPubKey (`20<pubkey>ac`);
`buildPlainSig(sig65)` wraps a schnorr sig for a plain input. Both are self-contained,
don't touch the proven owner-signed path.

**Panel wired (commit `70e9f96`):** 5 fields (seller pubkey, txid, index, parcel phoques,
listed price). Auto-funding-UTXO picker queries the buyer's P2PK address, picks the smallest
sufficient UTXO (must cover 100M payment + 900k fee). Build→sign→Submit, same pattern as
send/list/split. No descriptor emit (buyer already has the token).

**Proven live — full claim lifecycle from the web wallet:**
- `d941a074` — claim 3000-phoque listing from buyer1 (`8c02c20a…`)
- `a8bc5081` — list claimed parcel for sale at 500k sompi
- `77d595d8` — self-claim (seller=buyer), proving round-trip

Known limitation: payment hardcoded at `CLAIM_PAYMENT` (100M sompi). Listings priced above
100M will fail at build time; fix is `max(price, CLAIM_PAYMENT)` in `buildClaimTx`.

### NODE ADDR SWEEP — DONE (2026-06-27)

All 14 canonical `.rs` harnesses in `rust-examples\` had stale machine IPs baked into
`const NODE` — 12 with `192.168.0.206` (MAINNET box, the dangerous ones) and 2 with
`192.168.0.104`. These don't "fail loudly"; they SILENTLY connect to the wrong node.
All 14 repointed to `127.0.0.1:16210` in a single PowerShell sweep, verified clean via
`findstr`, committed as `e3f6628`.

Web wallet's `WRPC_URL` now `wss://node.outlawpolitics.org` (updated during VPS deploy).

**Rule going forward:** every `.rs` `NODE` const = `127.0.0.1:16210`, no exceptions.
Loopback is self-correcting: "this box's node," no addr to go stale, and if the node
isn't running it fails loudly (connection refused). Machine IPs (`.206`/`.104`/`.118`)
belong in documentation (this doc's TN10 GROUND FACTS), never compiled into a harness.

**Stale-clone divergence gap (known, not yet swept):** the sweep fixed CANONICAL
(`C:\bitphoque\rust-examples\`), but the D: build-clone binaries (`D:\…\silverscript\…\
src\bin\`) still carry old IPs until recopied+rebuilt. Proven live: `inspect_v3` connected
to `.104` on first run after the sweep because the build clone hadn't been updated; recopy
fixed it. **Discipline: recopy canonical→clone before running any harness from the D: tree.**

### Merge — PANEL WIRED + PROVEN LIVE (2026-06-27)

`buildMergeTx` in `bitphoque_tx.js` (commit `25deb04`). 2-input leader/delegate merge —
same-owner, not-for-sale parcels combined into one:

- **Input 0 (leader):** covenant UTXO, owner-signed at index 0. Full ABI sigscript
  (newStates, sigs, witnesses) + 2793B redeem. Sigscript 2989 bytes. computeBudget 24.
- **Input 1 (delegate):** covenant UTXO, owner-signed at index 0 (leader's sighash,
  NOT its own). Empty-args ABI + 2793B redeem. Sigscript 2797 bytes. computeBudget 24.
- **Output 0:** merged parcel → same owner, sum of both amounts, forSale=false.
- **Fee:** 1.3M sompi (two 2793B script reveals, consensus #10).

**Sighash crosscheck:** `mergeSelfTest()` rebuilds the exact merge tx from a CLI dry-run
(buyer2 merging 4000 + 16301 phoques, inputs `b4ab26d2…:1` + `b52e19c6…:0`) and asserts
leader sighash = `f8e741824120ad22ca334be7be3cf892412bdc449fd37b7e240b69afa4dc6e6e`.
**PASS** — byte-identical to Rust engine.

**CLI proven live:** tx `6d0ed811…` — buyer2 merged 4000 + 16301 → 20301 phoques. ACCEPTED.

**Web wallet proven live (commit `b04c041`):** split 1000-phoque parcel into 500+500
(tx `20641359…`), then merged back to 1000 (tx `128bb090…`). Full round-trip from UI.

**Parcel persistence fix (same commit):** localStorage read/write at all 4 mutation
sites (init load, refreshBalance prune, receiveParcel push, resetAll wipe). Also fixed
`refreshBalance` to use dynamic `await import('./bitphoque_p2sh.js')` instead of
`window.KASPA.deriveP2SHAddress` (which was never assigned on the KASPA object).

### Claim harness EngineFlags fix (2026-06-27)

`bitphoque_sale_claim_v3.rs` was written against the 2-field `EngineFlags` and failed to
compile on the unified 3-field engine. Fixed: added `zk_hardening_enabled: true` to both
`EngineFlags` initializers (lines 111 and 224), matching the CLI (`bitphoque_cli.rs` sets
`true` in all 8 call sites). Also added a diagnostic `println!` for the buyer sighash
(used to capture the `de52d220…` reference for `claimSelfTest`). Fix applied to canonical
and copied to D: clone. Harness compiles + dry-runs clean on the 3-field engine.

### TOOLCHAIN SPLIT — RESOLVED + GREEN-PATH PROVEN LIVE (2026-06-26)

The CLI now compiles, the 3-field engine is PROVEN to match on-chain, AND the full live
send→receive loop ran end-to-end. **GREEN-PATH DONE:** CLI `send` buyer2→web-wallet
(whole 1000-phoque parcel, no change) broadcast ACCEPTED as tx
`f8f133633960e3e693880adbd88f7db86bd591c7b41fdb169539fb39a030a82f`; recipient descriptor
pasted into the web Receive panel → **PARCEL CONFIRMED + TRACKED** (1000 phoques, owner
`b016a47b…`, outpoint `f8f13363…:0`, forSale=false). First trustless send→receive on TN10,
no coordinator: descriptor = claim, chain = truth, verified via amount-derived P2SH.
The wallet is now a working spending wallet, both halves proven live.

**1000-phoque parcel on-chain status:** confirmed live via `inspect_v3` (2026-06-27):
FOUND at `f8f13363…:0`, value 824,998,025,000 sompi. Parcel "disappeared" from the wallet
because tracked parcels are in-memory (page reload wipes them). The chain is fine; the
wallet just needs to re-onboard + refreshBalance to re-discover.

**Build clone is now `D:\Tocatta_Projects\kaspa-tn12\silverscript`** (C:\kaspa-tn12 retired —
renamed `C:\_RETIRED_kaspa-tn12`, the stale 2-field tree). Engine source-of-truth:
`D:\Tocatta_Projects\kaspa-tn12\rusty-kaspa` (the 3-field engine, `lib.rs:127-129` defines
`covenants_enabled / zk_hardening_enabled / sigop_script_units`).

**Root cause of the split:** silverscript pulled `kaspa-*` crates from GitHub branch `tn12`
(`1.1.1-toc.1`, the 2-field engine) while a 3-field local rusty-kaspa sat unused next to it.
Result: two copies of every kaspa crate in the graph → `Prefix`/`Address`/`ScriptPublicKey`/
`VerifiableTransaction` type mismatches (46 errors).

**Fix (in `D:\…\silverscript\Cargo.toml`):** single `[patch."https://github.com/kaspanet/
rusty-kaspa"]` block redirecting ALL kaspa crates (consensus-core, addresses, hashes,
txscript, txscript-errors, grpc-client, notify, rpc-core) to local `D:\…\rusty-kaspa` paths.
Workspace deps reverted to git coordinates so the patch is the single source redirect.
Member crate `silverscript-lang/Cargo.toml`: stripped `version = "1.1.1-toc.1"` pins from its
5 direct git deps (so local `1.2.1-toc.3` satisfies). Added `bs58 --features check`. Deleted
stale `Cargo.lock` (→ `.bak`) to force clean re-resolution. CLI + inspect_v3 + node_diag all
rebuilt GREEN on the unified `1.2.1-toc.3` engine.

**ENGINE VERIFIED LIVE (the gate that matters — clean build ≠ correct engine):**
`bitphoque_node_diag.exe` derived a known parcel P2SH and FOUND its live UTXO on-chain:
`[new 3000]` for-sale parcel at `kaspatest:pru2qyxy…` → outpoint `8dff6df4…:0`, value
97821666447 sompi. A FOUND is dispositive — can't accidentally derive a matching address for
a live UTXO. Local P2SH derivation matches on-chain ground truth.

- GREEN-PATH: DONE (see above). Descriptor emit is now PROVEN LIVE, not just built.
- Node synced at session end (`synced=true`, blocks=headers=1304873). Inspect NOT-FOUND
  results are now trustworthy (no index lag while synced).
- DEFERRED cleanup: physically delete/move `C:\_RETIRED_kaspa-tn12` once its file handle
  releases (was Defender/indexer-locked; rename succeeded, cross-drive move didn't).

### Genesis vs parcel P2SH (net-new finding, 2026-06-26)

Genesis uses **route-1**: `blake2b(entire compiled script)` → P2SH. Parcels use **route-2**:
template splice with fixed-width operands → `blake2b` → P2SH. These produce different scripts and
different addresses. The wallet only derives parcel addresses (route-2), which is correct. Do NOT
expect `deriveP2SHAddress(genesis operands)` to match the genesis UTXO on-chain.

### What's next

- **VPS node sync:** wait for IBD to complete (`synced=true`), then the live site is fully
  operational — wallet can query balances and submit txs via `wss://node.outlawpolitics.org`.
- **Multi-entity testing:** Entity1–4 set up (see TN10 GROUND FACTS). Test cross-entity
  send, claim, and list flows from distinct devices/wallets.
- **Parcel picker UX** — dropdown from tracked parcels instead of manual txid entry.
- **SMS Twilio node-monitor** — deferred until topology settles (listener at 18210 exists).
- **Receive flow (PROVEN LIVE):** descriptor decode (`bitphoque_descriptor.js`, byte-identical to Rust codec, vendored `@scure/base` base58check) → amount-derived P2SH recompute (`bitphoque_p2sh.js`) → on-chain confirm (outpoint must exist with our covenant_id; amount self-certifies via amount-derived P2SH) → track. Wired into the Receive panel (`receiveParcel`). Trustless refusal of a spent parcel proven live on TN10 from both console and UI. The descriptor is a CLAIM; the chain is TRUTH.
- **Descriptor emit (BUILT, UNTESTED LIVE):** send-submit assembles the recipient descriptor from `_pendingDesc` (recipientPubkey + sendAmount, stashed at build) + the submit-returned txid, via `encodeDescriptor`. Gated to send only (split/list/etc. don't set `_pendingDesc`). Code complete + committed; not yet run end-to-end (blocked on CLI toolchain, below).

---

## What this is (one sentence)

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 — a practice run for a
mainnet launch — built on the SAME stack as the Oracle Protocol (silverscript covenants, the
`binding=cov` N-to-M plumbing), deliberately NOT the indexer-based KRC-20 path. It is a principled
novelty token: phonetic play on "out of f**ks to give," seal (phoque) mascot, fixed supply, no
trusted coordinator. Site: https://www.outlawpolitics.org/BitPhoque/

---

## DESIGN — SETTLED

- **Architecture:** per-UTXO balance (real token, not a single-UTXO ledger). Transfers spend N
  token UTXOs and create M, conservation-enforced.
- **Ownership:** per-UTXO signature, mode `IDENTIFIER_PUBKEY=0x00` (plain checkSig).
- **Supply:** FIXED at genesis, NO minter. 42,069 phoques. `genesisIsMinter=false`, conservation
  always enforced. PROVEN, not asserted (reject-tests).
- **Subdivision — BitPups.** 1 phoque = 100,000,000 BitPups (8 dp). Pure denomination, no contract
  change. Mainnet genesis supply = 42,069 × 10^8 = 4,206,900,000,000,000 BitPups (within i64).
  TN10 token minted as whole units (no sub-unit headroom — needs re-genesis for mainnet).
- **Sale model (v3):** per-UTXO `forSale` (bool) + `price` (int). Listing = owner-signed. Claim =
  payment-authorized (no seller key needed). F1 double-claim fix: strict-increasing payment-output
  indices. Proven on-chain at 2×2 + 3×3 at VM.
- **Wallet custody:** non-custodial, BIP-39 12-word, key in-browser, never on server.
- **Receive problem:** solved by off-chain signed parcel descriptor (trustless truth) + optional
  indexer for UX. Descriptor: 118B fixed-width, base58check, Option-B format. Transport carries
  logical values; CScriptNum encoding only at derivation.
- **KAS handling:** "gas-tank, not vault." Fee floors: 700k list/split, 900k claim, 1.3M merge.
- **Why covenant, not KRC-20:** KRC-20 is inscription-based, interpreted by a trusted off-chain
  indexer. Covenant token enforces semantics at the script layer with no coordinator.

---

## THE CONTRACTS

### Original token — `covenants/bitphoque.sil` (DEPLOYED)
Copied verbatim from `kcc20.sil`. Ctor (6 args): `KCC20(byte[32] genesisPk, int genesisAmount,
byte genesisIdentifierType, bool genesisIsMinter, int maxCovIns, int maxCovOuts)`.
State: ownerIdentifier(byte[32]) + identifierType(byte) + amount(int) + isMinter(bool).
ABI: `__leader_transfer(newStates, sigs, witnesses)` + `__delegate_transfer()`.

### Sale token v3 — `covenants/bitphoque_sale_v3.sil` (LIVE)
Extends with forSale/price. Ctor (8 args): adds `bool genesisForSale, int genesisPrice`.
Script 2793 bytes. covenant_id `3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458`.
Caps 3/3 baked at genesis. F1 fix: strict-increasing payment-output indices.

### Compile command
```
cargo run --bin silverc -- C:\bitphoque\covenants\<file>.sil --ctor C:\bitphoque\covenants\<ctor>.json -c
```

---

## CONSENSUS RULES (the broadcast lessons)

1. v1/Toccata txs use ComputeBudget for input mass, NOT SigopCount.
2. v1 covenant spend costs 111,261 script units → ComputeBudget(11). (const COVENANT_SCRIPT_UNITS.)
3. Transfer fee floor ~378,600 (use 400k) — redeem script reveal inflates mass.
4. v1 needs ComputeBudget on EVERY input including plain P2PK.
5. Plain P2PK input costs ~100k script units in a covenant tx → ComputeBudget(11).
6. KIP-9 dust penalty: pay non-dust amounts (≥100M sompi for claims) to dodge storage mass.
7. v2 claim fee floor ~654,600 (use 700k).
8. **v3 list/split fee floor ~659k (use 700k).** 2793B script.
9. **v3 2-input claim fee floor = 857k (use 900k).** Heaviest single-covenant-input tx.
10. **v3 merge 2→1 fee floor = 1,244,500 (use 1.3M).** Two 2793B script reveals.
11. **v3 combined 2×2 claim cleared at 1.5M** (4 inputs, 6 outputs, two script reveals).

### Claim ABI detail (from proven harness + JS port)
- **Witness byte `[1]`** = payment output index. The covenant checks that output at
  this index pays ≥ price to the seller. Hardcoded because claim always has token/payment/change
  in outputs 0/1/2.
- **Dummy sig `[0u8; 65]`** = slot filler for the payment-authorized branch. The for-sale
  contract ignores the sig when witnesses indicate a payment-authorized claim.
- **P2PK scriptPubKey** for plain inputs/outputs = `20<32-byte-xonly>ac`
  (OP_DATA32 + pubkey + OP_CHECKSIG). New helper `makeP2pk` in `bitphoque_tx.js`.
- **computeBudget** per input: 24 for covenant input (240k script units), 11 for plain P2PK.

### Leader/delegate ABI (multi-covenant-input txs)
- **Input 0 = LEADER:** carries newStates[], sigs[], witnesses[] for ALL covenant inputs.
- **Other inputs = DELEGATE:** empty args, push own redeem script only.
- **Signed multi-input:** ALL sigs computed against leader input's sighash (index 0).
  Signing at the delegate's own index → VerifyError.
- witnesses: a single byte[] blob (e.g. `[0, 1]`), NOT separate pushes.

### Sigscript structure (ABI — proven via CLI diagnostic hex dump)
State[] is **transposed**: each field pushed separately as a concatenated fixed-width blob.
Field order: ownerIdentifier(32B), identifierType(1B), amount(8B LE), isMinter(1B),
forSale(1B), price(8B LE). Then sig[](65B each), byte[] witnesses, selector (OP_0=leader,
OP_1=delegate), redeem script push (OP_PUSHDATA2 + u16le(2793) + 2793B).

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
computeBudget=24 for 240k script units. scriptPublicKey = version(u16 hex) + script(hex).
For claim: input 0 computeBudget 24, input 1 computeBudget 11. Output 1+2 have
`covenant: null` (plain P2PK, not covenant-bound).

### Node / indexing quirks
- **Address-index lags tx-index by a few seconds** after confirmation. Mitigation: retry
  (3 attempts, 3s apart — implemented in `refreshBalance` and CLI inspect).
- **No by-outpoint UTXO query exists** on the RpcApi. `get_utxos_by_addresses` is the ONLY
  UTXO lookup; filter by outpoint client-side.
- Under heavy mempool load (60k+), lag can extend. Probe node health + a known-old parcel
  before suspecting a tx bug. `bitphoque_node_diag.rs` is the keep-around diagnostic.

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
    (v1, v2, ctors, slim experiments — kept for reference)
  rust-examples\
    bitphoque_cli.rs                   <- CLI: all 9 subcommands, feature-complete
    bitphoque_genesis_establish.rs     <- genesis harness (NODE const = 127.0.0.1)
    bitphoque_descriptor_v1.rs         <- parcel descriptor codec
    bitphoque_node_diag.rs             <- node health probe
    bitphoque_inspect_v3.rs            <- generalized on-chain inspect (retry, pubkey-only)
    bitphoque_sale_claim_v3.rs         <- claim harness (3-field EngineFlags, sighash print)
    (+ reject tests, sale harnesses, keygen, merge, split-transfer — all canonical)
  web\
    wallet.html                        <- the wallet (onboarding + HOME + action panels)
    bitphoque_p2sh.js                  <- in-browser P2SH derivation (template splice)
    bitphoque_sighash.js               <- sighash (keyed blake2b, proven vs Rust vectors)
    bitphoque_sigscript.js             <- ABI sigscript builder (proven vs CLI hex dump)
    bitphoque_tx.js                    <- tx builder: send/split/list + buildClaimTx + selftests
    bitphoque_descriptor.js            <- parcel descriptor codec (decode + encode)
    vendor/                            <- @scure/bip39, @scure/bip32, @noble/curves,
                                          @noble/hashes — all v2.2.0, vendored locally
  docs\
    bitphoque_status.md                <- this file
    bitphoque_history.md               <- on-chain tx ledger + session history
    bitphoque_audit_2026-06-25.md      <- security audit
    bitphoque_swap_covenant_design.md  <- sale/swap design + auction future-build
    bitphoque_mainnet_checklist.md     <- pre-mainnet gate
    vps_handoff.md                     <- VPS setup + operational reference
    operating_baseline.md              <- shared portable baseline
```

Own git repo (branch `master`). Not yet pushed to GitHub.

---

## TN10 GROUND FACTS

- **Local node (Big Daddy):** `127.0.0.1:16210` gRPC, `18210` JSON wRPC. kaspad 2.0.1.
  Launch: `kaspad.exe v2.0.1 --testnet --netsuffix=10 --appdir=C:\kaspa-testnet10\datadir
  --utxoindex --disable-upnp --rpclisten=0.0.0.0:16210 --rpclisten-borsh=0.0.0.0:17210
  --rpclisten-json=0.0.0.0:18210`. Mainnet stays on .206, untouched.
- **VPS node:** Hetzner CX22 (Helsinki), `89.167.32.239`. kaspad 2.0.1, systemd service.
  Public endpoint: `wss://node.outlawpolitics.org` (Caddy TLS → `127.0.0.1:18210`).
  SSH: `ssh root@89.167.32.239`. Operational details in `vps_handoff.md`.
- **Build clone:** `D:\Tocatta_Projects\kaspa-tn12\silverscript` (the unified 3-field tree).
  `C:\kaspa-tn12` retired → `C:\_RETIRED_kaspa-tn12` (delete when file-handle releases).
  **Recopy canonical→clone before running any harness from D:**, because the sweep fixed
  canonical but cloned binaries may be stale (see NODE ADDR SWEEP above).
- **Fee floors:** genesis 200k; transfer 400k; list/split 700k; claim 900k; merge 1.3M; 2×2 claim 1.5M.

### Test entities (PUBLIC keys only — privkeys never in tracked files)

| Entity | Label | Device | Address | Pubkey (hex) | TKAS |
|--------|-------|--------|---------|-------------|------|
| 1 | phoque | webapp | `kaspatest:qzcpdfrm752k2fzh6n2uvll2mh37uex65840uwwnv2nu2nl09ylgkx8ra2rcr` | `1602d48f7ea2aca48afa9ab8cffd5bbc7dcc9b543d5fc73a6c54f8a9fde527d1` | ~2,999 |
| 2 | kasware | KasWare | same as Entity 1 (same seed) | same as Entity 1 | ~2,999 |
| 3 | phonePhoques | phone | `kaspatest:qzhl95s2l85kxyflfm2kk3gx80ap9da0wcrt9ecndnwqu0vg9jeszusfgmz20` | `15fe5a415f3d2c6227e9daad68a0c77f4256f5eec0d65ce26d9b81c7b1059660` | 1,000 |
| 4 | tabletphoques | tablet | `kaspatest:qpx2a6yxk6lg5shj6mv8tf4884ua0vyy550pg95q4wckkea092p4k38nn06lt` | `0995dd10d6d7d1485e5adb0eb4d4e7af3af61094a3c282d015762d6cf5e5506b` | 1,000 |

Entity 1+2 share a seed — webapp and KasWare derive the same key, used to verify HD
derivation cross-implementation. Entities 3+4 are genuinely separate keys for multi-party
testing (send, claim, list between distinct holders on distinct devices).

**Legacy key labels** (from CLI/harness development — still referenced in harness source and
`.key` files): buyer1 `8c02c20a…`, buyer2 `5d338aa5…`,
buyer3 `267496ce01f2a35653491c61b56b23e6b7dbe777a02b146286ac3635ff36ab61`.

- **Faucet:** min 1000 TKAS/request, 24h cap 100k.
- **Isolation lesson:** one box, one node, one datadir, one launch script. .206 runs mainnet
  flawlessly because it runs nothing else. Apply this discipline to every node.
- **NODE addr rule:** every `.rs` `const NODE` = `127.0.0.1:16210`. No machine IPs (`.206`/
  `.104`/`.118`) compiled into any harness. Loopback is self-correcting. Machine IPs are
  documentation-only (here and in launch flags).

---

## REFERENCE (adapted sources)

- `kcc20.sil` — token contract. `kcc20-minter.sil` — deliberately NOT used.
- `covenant_escrow.sil` / `covenant_mecenas.sil` — output-payment-inspection patterns.
- `kcc20_tests.rs` + `common.rs` — leader/delegate sigscript, local-test pattern.
- `crypto/txscript/src/lib.rs` + `script_builder.rs` — element cap + with_flags.
- `consensus/core/src/mass/mod.rs` — calc_storage_mass, ComputeBudget, script-units.
- `consensus/core/src/hashing/sighash.rs` — calc_schnorr_signature_hash (reimplemented in JS).
- `consensus/core/src/hashing/hashers.rs` — TransactionSigningHash key = `b"TransactionSigningHash"`.
- `silverscript-lang/src/compiler/mod.rs` — build_sig_script_for_covenant_decl.

---

## PARKED DESIGN — see `bitphoque_swap_covenant_design.md`

- **#1 Gas-tank fee sizing** (buyer 900k/claim, lister 700k/listing).
- **#2 Wallet sends pups + dust** (derive dust floor from KIP-9 before hardcoding).
- **#3 Marketing audience** (Kaspa-native / meme-culture / self-erecting-thesis).
- **#4 "Boques"** = colloquial for phoques; completes phoque/pup/boque denomination story.
- **#5 Price-escalating emission** = step bonding curve; design fork A/B/Hybrid.
- **#6 Wallet clarity** (KAS vs phoque disambiguation, unit-confusion audit, "how to acquire KAS").
- **#7 Key naming** — legacy buyer1/buyer2/buyer3 labels remain in harness source + `.key`
  filenames. Entity1–4 labels are the current test-entity names (see GROUND FACTS). Full
  rename sweep of harness references deferred for a quiet moment.

---

## OPEN ITEMS

- [x] Receive flow (descriptor decode → confirm → record) — PROVEN LIVE
- [x] Claim panel — wired + proven live (d941a074, a8bc5081, 77d595d8), commit `70e9f96`
- [x] Claim on-chain — three web-wallet txs, full lifecycle (claim→list→claim)
- [x] Merge panel — wired + proven live (split 206413…, merge 128bb090…), commit `b04c041`
- [x] Tracked-parcel persistence — localStorage, saveParcels() at all mutation sites (commit `b04c041`)
- [x] VPS node provisioned — Hetzner CX22, kaspad + Caddy, `wss://node.outlawpolitics.org`
- [x] Site published to cPanel — `outlawpolitics.org/BitPhoque/`
- [ ] VPS IBD completion — wait for `synced=true`, then live site is fully operational
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
- [ ] Update shared panel banner ("Prototype — node-free…") — now false for wired panels
