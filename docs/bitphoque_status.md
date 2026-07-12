# BitPhoque — Status

_Read `operating_baseline.md` alongside this. BitPhoque lives in `C:\bitphoque` with its own git repo, separate from oracle-protocol. History + on-chain tx ledger in `bitphoque_history.md`._

---

## WHERE WE ARE (2026-07-12)

Feature-complete spending wallet on TN10. Every layer proven live. Mainnet blocked on SilverScript V1 (experimental, zero releases).

**Current blockers:**
- **SilverScript V1 release** — no timeline. Everything mainnet-bound (re-genesis, v4 compile, downstream sync) is gated on this.
- **CLI import failures** — the only Cargo project with `bitphoque_cli` wired is `C:\_RETIRED_kaspa-tn12\rusty-kaspa` (v1.2.1-toc.3, TN12-era). All import attempts return NOT FOUND. Likely cause: TN12 SilverScript compiler derives different P2SH addresses than what's on TN10. Need to either confirm network match or wire the CLI into a TN10-compatible rusty-kaspa build.
- **`getUtxosByCovenantId` RPC** — feature request drafted, not filed on kaspanet/rusty-kaspa. Would eliminate the parcel registry as a hard dependency.

**Recent completions (sig #012, commit `4e32af4`):**
Descriptor removed from happy path (Send/Sell/Receive). Sender's wallet registers recipient's output parcel with registry on confirm. Receives recorded in tx history. "Show recovery phrase" feature. Supply accounting confirmed: 42,069 phoques across buyer1 (25,301) + buyer2 (1,001) + buyer3 (15,767).

---

## WHAT THIS IS

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 — built on SilverScript covenants (`binding=cov` N-to-M), deliberately NOT the indexer-based KRC-20 path. Fixed supply (42,069 phoques), no trusted coordinator, self-erecting. Proving ground for the oracle protocol's covenant patterns.

Site: https://bitphoque.org/ — Repo: https://github.com/kyle4nia/BitPhoque

---

## STRATEGIC CONTEXT — KCC20 + ORACLE

BitPhoque's architecture maps directly to the emerging KCC20 standard (kas-smith forum):

- **State header:** `ownerIdentifier(32B) + identifierType(1B) + amount(8B)` = KCC20 standard header. `forSale(1B) + price(8B)` = extended state, which under the proposed Capsule/virtual-extension model would sit behind a digest.
- **Transfer method:** `__leader_transfer` + `__delegate_transfer` = KCC20 leader/delegator split with `prevStates[]`, `newStates[]`, `sigs[]`, `witnesses[]`.
- **Token descriptor:** the spec's `TokenDescriptor { prefix, suffix, state_layout, leader/delegator selectors, optional_extensions }` is a generalized version of what we maintain as separate modules (`bitphoque_p2sh.js`, `bitphoque_sigscript.js`, `bitphoque_tx.js`). If adopted, those modules collapse into a standard library.
- **Borrowed receive extension** (`0xFF` witness byte, additive-only): would reduce parcel fragmentation and merge frequency. Griefing vector flagged by Manyfest — outpoint-bound pre-signed listings invalidated by unsolicited borrowed receives. Per-UTXO opt-out needed.
- **Open ICC / Capsule pattern (Argent compiler):** the oracle's core architecture requirement. Oracle state as a Capsule (standard fields + virtual extension digest), consumer covenants compiled against the Capsule interface without knowing the oracle's internals. Design oracle state layouts as Capsules from day one.

---

## DESIGN — SETTLED

- **Architecture:** per-UTXO balance, conservation-enforced.
- **Ownership:** per-UTXO signature, `IDENTIFIER_PUBKEY=0x00` (plain checkSig).
- **Supply:** FIXED at genesis, NO minter. 42,069 phoques. `genesisIsMinter=false`.
- **Subdivision:** 1 phoque = 100,000,000 BitPups (8 dp). TN10 minted whole; mainnet needs re-genesis for sub-unit headroom.
- **Sale model (v3):** per-UTXO `forSale` + `price`. Listing = owner-signed. Claim = payment-authorized.
- **Wallet custody:** non-custodial, BIP-39 12-word, key in-browser, never on server.
- **Receive:** registry-based discovery (primary) + offline descriptor codec (fallback, 118B base58check).
- **KAS handling:** "gas-tank, not vault." Fee floors: 700k list/split, 900k claim, 1.3M merge.

---

## THE CONTRACTS

**Original token** — `covenants/bitphoque.sil` (DEPLOYED). State: ownerIdentifier + identifierType + amount + isMinter. ABI: `__leader_transfer` + `__delegate_transfer`.

**Sale token v3** — `covenants/bitphoque_sale_v3.sil` (LIVE). Extends with forSale/price. Script 2793 bytes. covenant_id `3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458`. Caps 3/3. F1 fix: strict-increasing payment-output indices.

**Sale token v4** — `covenants/bitphoque_sale_v4.sil` (READY, not compiled). Strips minter branch + identifierType. Mainnet target. Blocked on V1 compiler.

**Compile:** `cargo run --bin silverc -- C:\bitphoque\covenants\<file>.sil --ctor C:\bitphoque\covenants\<ctor>.json -c`

---

## CONSENSUS RULES

- v1/Toccata txs use ComputeBudget for input mass, NOT SigopCount.
- Covenant input: 111,261 script units → ComputeBudget(11). Plain P2PK → also ComputeBudget(11).
- KIP-9 dust penalty: ≥100M sompi for claims.
- Fee floors: genesis 200k; transfer 400k; list/split 700k; claim 900k; merge 1.3M; 2x2 claim 1.5M.

### ABI Reference

**Claim:** witness byte `[1]` = payment output index. Dummy sig `[0u8; 65]` for payment branch. P2PK scriptPubKey: `20<32B xonly>ac`. computeBudget: 24 (covenant), 11 (P2PK).

**Leader/delegate (multi-input):** Input 0 = leader (carries all args). Others = delegate (empty args + redeem push). ALL sigs against leader sighash (index 0).

**Sigscript structure:** State[] transposed: ownerIdentifier(32B), identifierType(1B), amount(8B LE), isMinter(1B), forSale(1B), price(8B LE). Then sig[], witnesses, selector (OP_0/OP_1), redeem push (2793B).

**submitTransaction JSON:**
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

**Node quirks:** address-index lags tx-index by seconds (3 retries, 3s apart). No by-outpoint UTXO query — `get_utxos_by_addresses` only, filter client-side.

---

## TN10 GROUND FACTS

- **Big Daddy (local):** `127.0.0.1:16210` gRPC, `18210` JSON wRPC. kaspad v2.0.1.
  Launch: `kaspad.exe v2.0.1 --testnet --netsuffix=10 --appdir=C:\kaspa-testnet10\datadir --utxoindex --disable-upnp --rpclisten=0.0.0.0:16210 --rpclisten-borsh=0.0.0.0:17210 --rpclisten-json=0.0.0.0:18210`
- **VPS node:** Hetzner CX33, `89.167.32.239`. `wss://node.outlawpolitics.org`. SSH: `ssh root@89.167.32.239`. Details in `vps_handoff.md`.
- **Mainnet node:** `kaspad-mainnet.service` on VPS, synced at DAG tip (confirmed 2026-07-11).
- **CLI build location:** `C:\_RETIRED_kaspa-tn12\rusty-kaspa` (v1.2.1-toc.3). Binary at `rothschild\src\bin\bitphoque_cli.rs`. **WARNING:** TN12-era build — P2SH derivation may not match TN10 on-chain state.
- **NODE addr rule:** every `.rs` `const NODE` = `127.0.0.1:16210`.
- **Faucet:** min 1000 TKAS/request, 24h cap 100k.

### Deployment

```
Browser → wss://node.outlawpolitics.org (443, TLS) → Caddy (VPS) → kaspad (ws://127.0.0.1:18210)
```

Static hosting: `bitphoque.org` → `/var/www/bitphoque/`; `testnet.bitphoque.org` → `/var/www/bitphoque-testnet/`.
Deploy: `cd /d C:\bitphoque\web && scp <file> root@89.167.32.239:/var/www/bitphoque/`

### Test entities

| Entity | Label | Pubkey (hex) | Notes |
|--------|-------|-------------|-------|
| 1 | phoque (webapp) | `1602d48f...` | ~2,999 TKAS |
| 3 | phonePhoques | `15fe5a41...` | 1,000 TKAS |
| 4 | tabletphoques | `0995dd10...` | 1,000 TKAS |

**CLI keys** (`.key` files, gitignored): buyer1 `6833fe15...` (25,301 phoques), buyer2 `6febe903...` (1,001 phoques, 2 UTXOs), buyer3 `e4ffa42f...` (15,767 phoques, 2 UTXOs). Owner key: `bitphoque_owner.key`.

### Genesis vs parcel P2SH
Genesis = route-1 (`blake2b(entire compiled script)` → P2SH). Parcels = route-2 (template splice → `blake2b` → P2SH). Different scripts, different addresses.

---

## FILE TREE (`C:\bitphoque`)

```
C:\bitphoque\
  .gitignore                           <- *.key + wallet.json
  bitphoque_owner.key                  <- original/funding signer (GITIGNORED)
  bitphoque_buyer{1,2,3}.key           <- CLI test keys (GITIGNORED)
  wallet.json                          <- CLI parcel state (GITIGNORED)
  covenants\
    bitphoque_sale_v3.sil              <- LIVE covenant source
    bitphoque_sale_v4.sil              <- mainnet target (not yet compiled)
    bitphoque_compiled_real.json       <- compiled contract + genesis ctor args
  rust-examples\
    bitphoque_cli.rs                   <- CLI: 9 subcommands, feature-complete
    bitphoque_inspect_v3.rs            <- on-chain inspect (retry, pubkey-only)
    (+ genesis, descriptor, node_diag, claim, reject tests, sale, keygen, merge, split)
  web\
    wallet.html                        <- spending wallet (v2, card-based UI)
    bitphoque_config.js                <- network config (testnet/mainnet flip)
    bitphoque_p2sh.js                  <- in-browser P2SH derivation
    bitphoque_sighash.js               <- sighash (keyed blake2b)
    bitphoque_sigscript.js             <- ABI sigscript builder
    bitphoque_tx.js                    <- tx builder + selftests
    bitphoque_descriptor.js            <- parcel descriptor codec
    vendor/                            <- @scure/bip39, @scure/bip32, @noble/curves, etc
  docs\
    bitphoque_status.md                <- this file
    bitphoque_history.md               <- on-chain tx ledger + session history
    bitphoque_audit_2026-06-25.md      <- security audit
    operating_baseline.md              <- shared portable baseline
    (+ wallet_v2_design, swap_covenant_design, mainnet_checklist, vps_handoff)
```

Git repo (master). Published at https://github.com/kyle4nia/BitPhoque

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

## OPEN ITEMS

### Blockers
- [ ] SilverScript V1 release (gates all mainnet work)
- [ ] CLI binary/network mismatch — wire CLI into TN10-compatible rusty-kaspa build, or confirm TN12 build works against TN10 node
- [ ] File `getUtxosByCovenantId` issue on kaspanet/rusty-kaspa (draft ready)

### Mainnet path
- [ ] v4 downstream code sync (sigscript_v4 done; descriptor OK; compiled_real/CLI/wallet blocked on V1 compile)
- [ ] Clear `bitphoque_mainnet_checklist.md`
- [ ] Re-genesis produces new `covenant_id` — sweep `3baed5a3...` in all JS + harnesses
- [ ] CLI .key files have no BIP-39 backup — mainnet genesis should mint to BIP-39 wallet key

### Cleanup / minor
- [ ] cPanel: evaluate whether still needed (outlawpolitics.org redirect)
- [ ] D: build-clone bin sweep (recopy 14 harnesses → clone, rebuild)
- [ ] Delete `C:\_RETIRED_kaspa-tn12` (after CLI migration)
- [ ] Buy transactions don't record seller as counterparty in tx history

### Parked
- Auction — separate future build (stateful/temporal). See `bitphoque_swap_covenant_design.md`
- SMS/Twilio node monitor
- Price-escalating emission (step bonding curve)
- Key naming — legacy buyer1/2/3 rename sweep

---

## SESSION SIGNATURES

_Append-only. Newest first. Prior sigs pushed to `bitphoque_history.md`._

---sig #012 | 2026-07-11 ~EST | desktop | trees: bitphoque | scope: descriptor UX overhaul, registry-based discovery, tx history receives, show recovery phrase, supply accounting, explorer/RPC feature request draft | head: 4e32af4

---sig #013 | 2026-07-12 ~EST | desktop | trees: bitphoque | scope: kas-smith forum analysis (KCC20 spec, borrowed receive, open ICC/Capsule/virtual state), forum post shipped, CLI import debugging (TN12/TN10 mismatch identified), operating baseline trimmed, status doc trimmed | head: pending
OPENING ASSESSMENT: BitPhoque feature-complete on TN10, mainnet blocked on SilverScript V1. Plan: forum analysis, CLI sends from buyer keys.
DRIFT: CLI import universally failed — discovered the only wired Cargo project is TN12-era (v1.2.1-toc.3), likely deriving wrong P2SH addresses for TN10. Pivoted from "do the sends" to "diagnose why nothing imports." Forum work was productive — shipped a post to kas-smith sharing BitPhoque's production findings, analyzed KCC20 spec alignment, borrowed receive implications, and open ICC/Capsule architecture relevance to the oracle project.
