# BitPhoque — Reference

_Stable reference. Lookup only — not read every session. Current state, blockers, and open items live in `bitphoque_status.md`. Update here only when the underlying facts change (new contract, consensus rule, node, or file-tree change)._

---

## ⚠ STALENESS BANNER (2026-10-14, sig #021)

The **architecture** sections below — STRATEGIC CONTEXT, THE CONTRACTS, and the CONSENSUS RULES / ABI Reference — describe the **pre-Argent SilverScript `.sil` design** (4-field state with isMinter, separate forSale/price, the v3/v4 `.sil` contracts, covenant_id `3baed5a3…`). That design is **SUPERSEDED** by the Argent / KCC20 conform in `bitphoque_status.md` DESIGN: standalone single-covenant KCC20 asset, 6-field state, no minter, on-chain sale via extension_commitment + set_price/claim. **Read status DESIGN for the current contract.**

Still valid below: the TN10/VPS node facts, test entities, `.key` inventory, deployment/hosting, and file tree — EXCEPT the local-node and CLI facts are **big-daddy-local** and must be re-established on pyt (see the CLI-rebuild open item).

## TOOLCHAIN PINS (Argent — 2026-10-14, sig #021)

- argent + argent-runtime @ `github.com/argent-lang/argent` rev `9a9f4b107116d0b259fae629f200fa3b663e7e9d` (short `9a9f4b10`)
- rusty-kaspa (kaspa-consensus-core + kaspa-txscript) @ `github.com/kaspanet/rusty-kaspa` rev `a41a333b08848f41bf737b72592e463a6011b8ac` (short `a41a333b`)
- Rust: **1.94.1 floor** (pinned in the reference's `rust-toolchain.toml`); pyt local 1.99.0 clears it
- Reference repo: `github.com/argent-lang/kcc20-reference` @ `c8a0871` (2-commit repo; `cargo test --locked` 60/60 — proven on KarrrlsBigDaddy, re-proven on pyt 2026-10-14, MSVC host)
- **PIN-ASSERT (Argent):** before any build that will touch TN10, assert the scaffold's `Cargo.toml` still carries argent `9a9f4b10` + rusty-kaspa `a41a333b` — the deploy binary must match the TN10 node. A moved pin ⇒ re-prove 60/60 before trusting it.

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
    bitphoque_status.md                <- WHERE we are (volatile, read every session)
    bitphoque_reference.md             <- this file (stable, lookup only)
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
