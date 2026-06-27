# BitPhoque — Status

_Read `operating_baseline.md` alongside this doc. BitPhoque lives in its own tree (`C:\bitphoque`)
and its own git repo — separate from oracle-protocol. End-of-session checkpoint covers both trees._

_The on-chain tx ledger (every confirmed txid) and session-by-session history live in
`bitphoque_history.md`. Start here; reach for history only to look up a past fact._

---

## WHERE WE ARE (2026-06-26)

The web wallet is a **spending wallet**. Every layer of the write seam is proven against ground
truth and live on TN10:

- **Onboarding:** BIP-39 12-word, real entropy (`@scure/bip39`), AES-GCM encrypted persistence,
  HD key derivation (`m/44'/111111'/0'/0/0` → secp256k1 x-only → Kaspa bech32), cross-checked vs
  KasWare.
- **Read side:** `refreshBalance()` wired — per-parcel P2SH derivation (template splice,
  `bitphoque_p2sh.js`) → `getUtxosByAddresses` over JSON wRPC (`ws://127.0.0.1:18210`) → covenant
  filter → reconcile + prune.
- **Write side (PROVEN LIVE):** sighash (`bitphoque_sighash.js`, keyed blake2b, validated vs Rust
  test vectors) → schnorr sign (`@noble/curves`) → ABI sigscript (`bitphoque_sigscript.js`,
  transposed State[], validated vs CLI hex dump) → JSON submit via `wrpcCall("submitTransaction")`.
  **Live mined tx `4f3d791f`** — wallet sent 1000 phoques, node accepted.
- **Panels wired:** send, split, list, partial-list — all real build→sign→submit. Claim and merge
  still stubs (multi-input, different signing patterns).
- **CLI:** feature-complete — import / balance / list / split / merge / partial-list / claim / send /
  receive. All proven on-chain.
- **Audit:** `bitphoque_audit_2026-06-25.md` — history-blind source reconstruction, no fund-loss
  or supply-break vector found. R3 (minter branch annotated) and R5 (genesis node const repointed)
  closed. R1 (F-2 genesis derivation) retired by design (genesis uses route-1 whole-compiled-script,
  parcels use route-2 splice — intentionally different, see below).

### TOOLCHAIN SPLIT — RESOLVED + ENGINE VERIFIED (2026-06-26)

The CLI now compiles and the 3-field engine is PROVEN to match on-chain. Green-path is
unblocked from the toolchain side; only gated now on the node finishing IBD.

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

**Stale node-addr trap (fixed):** build-tree `bitphoque_node_diag.rs` had `192.168.0.206`
(MAINNET box!) baked in — recopied canonical (`127.0.0.1:16210`) over it. Watch for `.206`/
`.104` in any build-clone binary; canonical is always `127.0.0.1:16210`.

- ACTION next session (gated on `synced=true`): CLI `send` buyer1→web-wallet pubkey
  `b016a47bf515652457d4d5c67feadde3ee64daa1eafe39d362a7c54fef293e8b`, paste emitted
  descriptor into the web Receive panel → expect first green PARCEL CONFIRMED.
- Node was mid-IBD at session end (`synced=false`, blocks 1250939 / headers 1283913,
  ~33k behind). Do NOT trust inspect NOT-FOUND results until `synced=true` (index lag).
- DEFERRED cleanup: physically delete/move `C:\_RETIRED_kaspa-tn12` once its file handle
  releases (was Defender/indexer-locked; rename succeeded, cross-drive move didn't).
### Genesis vs parcel P2SH (net-new finding, 2026-06-26)

Genesis uses **route-1**: `blake2b(entire compiled script)` → P2SH. Parcels use **route-2**:
template splice with fixed-width operands → `blake2b` → P2SH. These produce different scripts and
different addresses. The wallet only derives parcel addresses (route-2), which is correct. Do NOT
expect `deriveP2SHAddress(genesis operands)` to match the genesis UTXO on-chain.

### What's next

- **Receive flow (PROVEN LIVE):** descriptor decode (`bitphoque_descriptor.js`, byte-identical to Rust codec, vendored `@scure/base` base58check) → amount-derived P2SH recompute (`bitphoque_p2sh.js`) → on-chain confirm (outpoint must exist with our covenant_id; amount self-certifies via amount-derived P2SH) → track. Wired into the Receive panel (`receiveParcel`). Trustless refusal of a spent parcel proven live on TN10 from both console and UI. The descriptor is a CLAIM; the chain is TRUTH.
- **Descriptor emit (BUILT, UNTESTED LIVE):** send-submit assembles the recipient descriptor from `_pendingDesc` (recipientPubkey + sendAmount, stashed at build) + the submit-returned txid, via `encodeDescriptor`. Gated to send only (split/list/etc. don't set `_pendingDesc`). Code complete + committed; not yet run end-to-end (blocked on CLI toolchain, below).
- **Parcel picker UX** — dropdown from tracked parcels instead of manual txid entry.
- **Claim panel** — multi-input, payment-authorized (dummy sig + witness index). Different
  sigscript: covenant input unsigned, buyer's plain input signed at its own index.
- **Merge panel** — multi-input, leader/delegate (two sigscripts, two 2793B script reveals).
- **SMS Twilio node-monitor** — deferred until topology settles (listener at 18210 exists).
- **Publish** — to the live cPanel host once receive + claim are wired.

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
    (+ reject tests, sale harnesses, keygen, merge, split-transfer — all canonical)
  web\
    wallet.html                        <- the wallet (onboarding + HOME + action panels)
    bitphoque_p2sh.js                  <- in-browser P2SH derivation (template splice)
    bitphoque_sighash.js               <- sighash (keyed blake2b, proven vs Rust vectors)
    bitphoque_sigscript.js             <- ABI sigscript builder (proven vs CLI hex dump)
    bitphoque_tx.js                    <- tx builder (sighash + sign + sigscript + JSON)
    vendor/                            <- @scure/bip39, @scure/bip32, @noble/curves,
                                          @noble/hashes — all v2.2.0, vendored locally
  docs\
    bitphoque_status.md                <- this file
    bitphoque_history.md               <- on-chain tx ledger + session history
    bitphoque_audit_2026-06-25.md      <- security audit
    bitphoque_swap_covenant_design.md  <- sale/swap design + auction future-build
    bitphoque_mainnet_checklist.md     <- pre-mainnet gate
    operating_baseline.md              <- shared portable baseline
```

Own git repo (branch `master`). Not yet pushed to GitHub.

---

## TN10 GROUND FACTS

- **Node:** Big Daddy (`127.0.0.1:16210` gRPC, `18210` JSON wRPC). kaspad 2.0.1.
  Launch: `kaspad.exe v2.0.1 --testnet --netsuffix=10 --appdir=C:\kaspa-testnet10\datadir
  --utxoindex --disable-upnp --rpclisten=0.0.0.0:16210 --rpclisten-borsh=0.0.0.0:17210
  --rpclisten-json=0.0.0.0:18210`. Mainnet stays on .206, untouched.
- **Build clones:** rusty-kaspa `C:\kaspa-tn12\rusty-kaspa`; silverscript `C:\kaspa-tn12\silverscript`.
- **Fee floors:** genesis 200k; transfer 400k; list/split 700k; claim 900k; merge 1.3M; 2×2 claim 1.5M.
- **Key pubkeys:** buyer1 `8c02c20a…`, buyer2 `5d338aa5…`, buyer3 `267496ce…`.
- **Faucet:** min 1000 TKAS/request, 24h cap 100k.
- **Isolation lesson:** one box, one node, one datadir, one launch script. .206 runs mainnet
  flawlessly because it runs nothing else. Apply this discipline to every node.

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

---

## OPEN ITEMS

- [ ] Receive flow (descriptor decode → confirm → record)
- [ ] Claim panel (multi-input, payment-authorized)
- [ ] Merge panel (multi-input, leader/delegate)
- [ ] Parcel picker UX (dropdown from tracked parcels)
- [ ] SMS Twilio node-monitor
- [ ] Publish to live host
- [ ] Original-token zero-pubkey owner quirk (observed, not blocking)
- [ ] Auction — separate future build (stateful/temporal)
- [ ] Mainnet: clear `bitphoque_mainnet_checklist.md`
- [ ] Make public on GitHub (clean repo name + README + secret-sweep)
