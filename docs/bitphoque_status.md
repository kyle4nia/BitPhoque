# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working
discipline applies: one command at a time, single-line terminal commands, plain-English-before-code,
source-of-truth files committed to git, dry-run before broadcast, verify-by-reading-source for
anything that enforces, local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`)
and its OWN git repo — separate from oracle-protocol. The end-of-session git checkpoint ritual now
covers TWO trees._

_Last updated: BASELINE COMMITTED. The KCC20 reference token contract compiles clean, unmodified,
in the tn10-toc3 silverc. Design settled, genesis config chosen. Resume point = build the genesis
establish harness (first .rs file)._

---

## What this is (one sentence)

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 — a practice run for a
mainnet launch — built on the SAME stack as the Oracle Protocol (silverscript covenants, the
`binding=cov` N-to-M plumbing), deliberately NOT the indexer-based KRC-20 path. It is a principled
novelty token: phonetic play on "out of f**ks to give," seal (phoque) mascot, fixed supply, no
trusted coordinator. Site: https://www.outlawpolitics.org/BitPhoque/ — Family Guy S8E18
"Quagmire's Dad" is the cultural reference (Chris's French homework: "phoque" = seal).

---

## Why covenant token, not KRC-20 (the model decision)

KRC-20 is inscription-based: data embedded in txs, interpreted by a trusted off-chain indexer
(Kasplex). That is the OPPOSITE of self-erecting — token truth depends on a coordinator agreeing on
state. A covenant token enforces token semantics (conservation, ownership) at the SCRIPT layer with
no coordinator, on the same rails as OracleRep. It is the honest BitPhoque and the better mainnet
rehearsal. KRC-20-via-inscription was explicitly rejected.

NOTE: this is NOT blocked on OpCheckSigFromStack (CSFS / silverscript #122, the thing OracleRep v4
waits on). An OWNED token transfer uses plain `OpCheckSig` over the spending tx ("this owner
authorizes THIS tx") — which already works. The admin-checkSig model abandoned as WRONG for
OracleRep's portable verdicts is exactly RIGHT for a token transfer.

---

## Design — SETTLED

- **Architecture:** per-UTXO balance (real token, not a single-UTXO ledger). Each token UTXO carries
  its own balance; transfers spend N token UTXOs and create M, conservation-enforced.
- **Ownership:** OWNED via per-UTXO signature. Three modes exist in the contract
  (`IDENTIFIER_PUBKEY=0x00`, `IDENTIFIER_SCRIPT_HASH=0x01`, `IDENTIFIER_COVENANT_ID=0x02`).
  **BitPhoque uses 0x00 (pubkey-owned)** — simplest, plain checkSig, no CSFS needed. The other two
  modes sit unused.
- **Supply:** FIXED at genesis, NO minter. **42,069 phoques** (TN10 placeholder — may pick a cleverer
  number before mainnet; it's just a genesis ctor arg, fully reversible). The whole supply is minted
  once into a genesis UTXO and can only ever be split / merged / transferred, never increased.
- **Self-erecting property:** satisfied by construction. With NO minter covenant deployed and every
  token UTXO genesis'd `isMinter=false`, conservation is ALWAYS enforced and supply is mathematically
  fixed. No privileged operator can inflate or freeze. Every holder is sovereign over their own UTXOs
  via their own key.

---

## The contract — `C:\bitphoque\covenants\bitphoque.sil`

Copied VERBATIM from the silverscript reference `kcc20.sil`
(`C:\kaspa-tn12\silverscript\silverscript-lang\tests\examples\kcc20.sil`). Faithful-adopt, NO fork.
Still self-identifies as `contract KCC20` — optional cosmetic rename to `BitPhoque` is a later edit
(will change compiled bytes; do AFTER baseline, hasn't been done yet).

**Why no code surgery for fixed-supply:** `checkAmounts` (conservation: `totalIn == totalOut`) and
`checkMintingTransfer` (forbids outputs setting `isMinter=true`) are BOTH gated `if(!isMinter)`. The
`isMinter` flag IS the switch that disables conservation. Genesis everything with `isMinter=false`
and there is no path to ever increase supply. Fixed supply = pure configuration, not a code change.
The companion minter contract `kcc20-minter.sil` is simply NEVER deployed / referenced.

### Constructor (6 args)
`KCC20(byte[32] genesisPk, int genesisAmount, byte genesisIdentifierType, bool genesisIsMinter, int maxCovIns, int maxCovOuts)`
- genesisPk: 32-byte owner (placeholder = 32 zeros in baseline; REAL key generated at genesis time)
- genesisAmount: 42069
- genesisIdentifierType: 0 (0x00 pubkey)
- genesisIsMinter: false
- maxCovIns / maxCovOuts: 2 / 2 (transfer fanout width — supports up to 2-in, 2-out txs)

### Per-UTXO state
`ownerIdentifier` (byte[32]) + `identifierType` (byte) + `amount` (int) + `isMinter` (bool).

---

## Baseline compile — DONE, COMMITTED (git 7cf5c95)

`bitphoque.sil` + `bitphoque_ctor.json` compile CLEAN, unmodified, in the tn10-toc3 silverc.
Captured output committed as `bitphoque_compiled_baseline.json` (frozen known-good bytecode, the
diff-target for any future edit — same idea as oracle-protocol's covenant_baseline.txt).

Key facts FROM the compile (verified by reading the lowered AST, not assumed):
- **state_layout: `{ start: 1, len: 46 }`** — each token UTXO's state is 46 bytes
  (32 owner + 1 identifierType + 8 amount + 1 isMinter + framing). This is BitPhoque's geometry,
  the analog of OracleRep v3's `{start:0, len:9}`.
- **ABI = TWO entrypoints (leader/delegate pattern):**
  * `__leader_transfer(newStates: State[], sigs: sig[], witnesses: byte[])`
  * `__delegate_transfer()` (no args)
  In a multi-input transfer, ONE input is the leader carrying the full new-state array; the others
  are delegates with empty sigscripts. This is NEW machinery vs OracleRep's single-in/single-out
  hops — it's where the harness work goes.
- **Conservation is auto-wired by the `binding=cov` macro.** The compiler-generated `__leader_transfer`
  reads each covenant input's state via `readInputState`, builds `prevStates[]`, calls the policy
  (checkSigs -> checkAmounts -> checkMintingTransfer), then `validateOutputState` per output. The
  N-to-M conservation we needed is generated; we did not hand-write it.
- ctor JSON encodings CONFIRMED working: `byte[32]` = array of `{"kind":"byte","data":N}`;
  bare `byte` = `{"kind":"byte","data":N}`; `bool` = `{"kind":"bool","data":false}`;
  `int` = `{"kind":"int","data":N}`.

### Compile command (from silverscript dir; source-of-truth .sil is in bitphoque)
```
cargo run --bin silverc -- C:\bitphoque\covenants\bitphoque.sil --ctor C:\bitphoque\covenants\bitphoque_ctor.json -c
```
Capture clean JSON to file (drops cargo noise): append ` > <path>.json 2>nul`.

---

## File tree (`C:\bitphoque`)
```
C:\bitphoque\
  covenants\
    bitphoque.sil                      <- the contract (KCC20 verbatim, canonical)
    bitphoque_ctor.json                <- 6-arg genesis ctor (42069 placeholder config)
    bitphoque_compiled_baseline.json   <- frozen known-good compiled output
  rust-examples\                       <- EMPTY. Canonical home for every .rs harness (always-save rule)
  docs\
    bitphoque_status.md                <- this file
```
Own git repo (root commit 7cf5c95). NOT yet pushed to GitHub — that's a later deliberate
"make it public" pass (clean repo name + README explaining the phoque joke + fixed-supply/no-coordinator
design), not a reflexive push. On-brand to go public eventually given the existing site/socials.

---

## RESUME POINT — genesis establish harness (the next real work)

The contract is proven; driving it on-chain is the work. Next session builds the GENESIS ESTABLISH
harness — the first `.rs` file (triggers the always-save rule: canonical copy to
`C:\bitphoque\rust-examples\` IN ADDITION to the disposable build location).

What it must do (model on OracleRep's `oracle_genesis_establish.rs` + THE GENESIS LESSON):
- [ ] Generate a REAL owner keypair (x-only 32-byte pubkey = genesisPk). Don't reuse a demo key.
- [ ] Build the ctor with the real pubkey (replace the 32-zero placeholder), 42069, type 0x00,
      isMinter false, 2, 2. Recompile -> real redeem script + P2SH address.
- [ ] THE GENESIS LESSON (carried from OracleRep, do NOT forget): a covenant UTXO CANNOT be created
      by sending funds directly to its P2SH address (-> covenant_id:None, unspendable). It MUST be
      ESTABLISHED as a bound output of a separate signed tx: spend a plain key-controlled input,
      create the token output, bind with `populate_genesis_covenants`. Two-step: establish, then
      transition.
- [ ] Dry-run before broadcast (gate real submission behind `--go`).
- [ ] Local VM verify (TxScriptEngine, covenants_enabled, CovenantsContext) BEFORE any TN10 broadcast.

Then, AFTER genesis works:
- [ ] TRANSFER harness — the hard part. Build an N->M transfer tx using the leader/delegate ABI:
      one input carries `__leader_transfer(newStates, sigs, witnesses)`, others are delegates. Sign
      each token input (checkSig over the tx). This generalizes OracleRep's single-input spender.
- [ ] REJECTION TESTS (non-negotiable, the only real proof the gate works): a tx that creates phoques
      from nothing MUST fail (conservation); a transfer with a wrong/missing owner signature MUST
      fail. The KCC20 reference test suite (`kcc20_tests.rs`) already proves these at the contract
      level — mirror that coverage at the harness level. Accept-AND-reject, not accept-only.
- [ ] Optional cosmetic: rename `contract KCC20` -> `BitPhoque` in the .sil (changes bytes; re-baseline).

### TN10 ground facts (from OracleRep, reusable)
- Node: PsychoNode at 192.168.0.206 — wRPC borsh 17210, gRPC 16210. Release tn10-toc3.
- Funding wallet: oracle-dev (kaspa-cli), ~98,999 TKAS left — can fund BitPhoque genesis input.
- Fee floor learned: ~167,100 sompi; use 200,000.
- rusty-kaspa clone at C:\kaspa-tn12\rusty-kaspa (tag tn10-toc3); silverscript at C:\kaspa-tn12\silverscript.

---

## Reference (the thing we adapted)
- `kcc20.sil` — the token contract (copied verbatim as bitphoque.sil).
- `kcc20-minter.sil` — the minter contract we deliberately DO NOT use.
- `kcc20_tests.rs` — the executable spec: split/merge/transfer/mint/burn + rejection tests. THE
  blueprint for the harnesses. Shows leader/delegate sigscript construction, owner signing, and the
  two-step genesis (handoff then split).
