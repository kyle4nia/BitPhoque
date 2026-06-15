# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working
discipline applies: one command at a time, single-line terminal commands, plain-English-before-code,
source-of-truth files committed to git, dry-run before broadcast, verify-by-reading-source for
anything that enforces, local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`)
and its OWN git repo — separate from oracle-protocol. The end-of-session git checkpoint ritual now
covers TWO trees._

_Last updated: **GENESIS IS LIVE ON TN10.** The genesis covenant UTXO was established and accepted
on-chain (tx f8d6d45a..., covenant_id 69262e04...). Contract proven, genesis broadcast + confirmed.
Resume point = build the TRANSFER harness (leader/delegate ABI + rejection tests)._

---

## What this is (one sentence)

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 — a practice run for a
mainnet launch — built on the SAME stack as the Oracle Protocol (silverscript covenants, the
`binding=cov` N-to-M plumbing), deliberately NOT the indexer-based KRC-20 path. It is a principled
novelty token: phonetic play on "out of f**ks to give," seal (phoque) mascot, fixed supply, no
trusted coordinator. Site: https://www.outlawpolitics.org/BitPhoque/ — Family Guy S8E18
"Quagmire's Dad" is the cultural reference (Chris's French homework: "phoque" = seal).

---

## GENESIS — LIVE ON TN10 (the milestone)

The genesis establishing transaction was built, signed, dry-run-verified, broadcast with `--go`,
ACCEPTED by the node, and confirmed mined (the funding UTXO is now spent/gone). BitPhoque exists.

- **tx id:** `f8d6d45a37f6049872bb12d55359a939e6677e6848ad6135a9e1fd2cbbb00875`
- **covenant_id:** `69262e04b63d1ea1008fd33f8aff38c08780056b40111d0db01a306ef11a766c`
  (bound by `populate_genesis_covenants` — came back `Some(...)`, NOT None: the genesis lesson worked)
- **owner pubkey (x-only):** `dc227307fd98d8190e75233f0e12cae00bdf412df643b06c005bc9d07e2432ce`
- **genesis covenant value:** 9,999,999,800,000 sompi (100,000 TKAS funding minus 200,000 fee)
- **spent funding outpoint:** `9f00751f3030b70e79776e54ded48397257915696200b37884ffd3c8ca622823:0`
- **real redeem script size:** **1540 bytes** (corrects an earlier ~740 estimate — the real
  compiled script is 1540B; matters for mass/fee on mainnet, irrelevant on TN10).
- **owner key:** TN10 PRACTICE KEY ONLY, permanently. Held in `C:\bitphoque\bitphoque_owner.key`
  (gitignored, never committed — verified clean). Was pasted in chat once during the practice run;
  that BURNS it by mainnet standards, which is fine for a throwaway testnet key. The mainnet owner
  key must be born air-gapped per the checklist and NEVER does this.

NOTE: genesis put the WHOLE funding balance into the single genesis covenant UTXO (full-minus-fee).
The 42,069 phoques live in that covenant UTXO at the token P2SH; "holding" them = controlling the
owner key. There is no wallet that shows a balance — query the P2SH on-chain to see them.

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
- genesisPk: 32-byte owner (placeholder = 32 zeros in baseline; REAL key = dc227307... at genesis)
- genesisAmount: 42069
- genesisIdentifierType: 0 (0x00 pubkey)
- genesisIsMinter: false
- maxCovIns / maxCovOuts: 2 / 2 (transfer fanout width — supports up to 2-in, 2-out txs)

### Per-UTXO state
`ownerIdentifier` (byte[32]) + `identifierType` (byte) + `amount` (int) + `isMinter` (bool).

---

## Baseline + real compile — DONE, COMMITTED

`bitphoque.sil` + `bitphoque_ctor.json` compile CLEAN, unmodified, in the tn10-toc3 silverc.
Frozen baseline committed as `bitphoque_compiled_baseline.json` (git 7cf5c95). The REAL compile
(owner pubkey baked in) is `bitphoque_compiled_real.json` + `bitphoque_ctor_real.json` (git 8a94cac)
— this is what the genesis harness reads its redeem script from at runtime.

Key facts FROM the compile (verified by reading the lowered AST, not assumed):
- **state_layout: `{ start: 1, len: 46 }`** — each token UTXO's state is 46 bytes
  (32 owner + 1 identifierType + 8 amount + 1 isMinter + framing). BitPhoque's geometry.
- **real redeem script: 1540 bytes** (confirmed at genesis runtime).
- **ABI = TWO entrypoints (leader/delegate pattern):**
  * `__leader_transfer(newStates: State[], sigs: sig[], witnesses: byte[])`
  * `__delegate_transfer()` (no args)
  In a multi-input transfer, ONE input is the leader carrying the full new-state array; the others
  are delegates with empty sigscripts. This is NEW machinery vs OracleRep's single-in/single-out
  hops — it's where the TRANSFER harness work goes (genesis did NOT need it).
- **Conservation is auto-wired by the `binding=cov` macro.** The compiler-generated `__leader_transfer`
  reads each covenant input's state via `readInputState`, builds `prevStates[]`, calls the policy
  (checkSigs -> checkAmounts -> checkMintingTransfer), then `validateOutputState` per output.
- ctor JSON encodings CONFIRMED: `byte[32]` = array of `{"kind":"byte","data":N}`;
  bare `byte` = `{"kind":"byte","data":N}`; `bool` = `{"kind":"bool","data":false}`;
  `int` = `{"kind":"int","data":N}`.

### Compile command (from silverscript dir; source-of-truth .sil is in bitphoque)
```
cargo run --bin silverc -- C:\bitphoque\covenants\bitphoque.sil --ctor C:\bitphoque\covenants\bitphoque_ctor_real.json -c
```
Capture clean JSON to file (drops cargo noise): append ` > <path>.json 2>nul`.

---

## The genesis harness (DONE — `rust-examples\bitphoque_genesis_establish.rs`, git d1b083b)

Adapted from OracleRep's `oracle_genesis_establish.rs`. Three changes from that model:
1. **Reads the redeem script at runtime** from `bitphoque_compiled_real.json` (no transcription of
   1540 bytes). Needs `serde_json` — was `cargo add serde_json -p rothschild` in the clone.
2. **Reads the signing key at runtime** from `C:\bitphoque\bitphoque_owner.key` (gitignored).
3. Self-funds from the BitPhoque P2PK address (faucet-funded), full-funding-minus-fee output.
Dry-run by default; `--go` to broadcast. Built as a rothschild bin in the clone; canonical copy in
`rust-examples\`. Build: `cargo build --release -p rothschild --bin bitphoque_genesis_establish`.

---

## File tree (`C:\bitphoque`)
```
C:\bitphoque\
  .gitignore                           <- ignores bitphoque_owner.key
  bitphoque_owner.key                  <- TN10 signing key (GITIGNORED, never committed)
  covenants\
    bitphoque.sil                      <- the contract (KCC20 verbatim, canonical)
    bitphoque_ctor.json                <- placeholder ctor (32-zero pubkey, baseline)
    bitphoque_ctor_real.json           <- REAL ctor (owner pubkey baked in)
    bitphoque_compiled_baseline.json   <- frozen known-good baseline bytecode
    bitphoque_compiled_real.json       <- REAL compiled contract (harness reads script from here)
  rust-examples\
    bitphoque_keygen.rs                <- owner keypair generator
    bitphoque_genesis_establish.rs     <- genesis harness (LIVE genesis sent with this)
  docs\
    bitphoque_status.md                <- this file
    bitphoque_mainnet_checklist.md     <- the pre-mainnet GATE
    operating_baseline.md              <- shared portable baseline (identical copy in oracle tree)
```
Own git repo (branch `master`). NOT yet pushed to GitHub — later deliberate "make it public" pass
(clean repo name + README explaining the phoque joke + fixed-supply/no-coordinator design).

---

## RESUME POINT — TRANSFER harness (the next real work)

Genesis is LIVE. The next milestone is moving phoques: build the TRANSFER harness using the
leader/delegate ABI. This is the HARD part genesis didn't need.

- [ ] TRANSFER harness — build an N->M transfer tx spending the genesis covenant UTXO. One input
      carries `__leader_transfer(newStates, sigs, witnesses)` (the full new-state array), others are
      delegates with empty sigscripts. Sign each token input (checkSig over the tx). Model on
      `kcc20_tests.rs` handoff/split flow (covenant_decl_sigscript, kcc20_state_array_arg, the
      leader/delegate split). The genesis covenant UTXO (covenant_id 69262e04...) is the input.
- [ ] First transfer = a "handoff" (move all 42069 to a new owner) or a "split" (e.g. 42069 -> two
      UTXOs). Handoff is simpler; do it first like the tests do.
- [ ] REJECTION TESTS (non-negotiable, the only real proof the gate works): a tx that creates phoques
      from nothing MUST fail (conservation); a transfer with a wrong/missing owner signature MUST
      fail. `kcc20_tests.rs` proves these at the contract level — mirror at the harness level.
      Accept-AND-reject, not accept-only.
- [ ] Local VM verify (execute_input_with_covenants / TxScriptEngine) BEFORE any TN10 broadcast.
- [ ] New `.rs` => always-save to `C:\bitphoque\rust-examples\`.

### Later / optional
- [ ] Cosmetic: rename `contract KCC20` -> `BitPhoque` in the .sil (changes bytes; re-baseline).
- [ ] SWAP COVENANT — the trustless stranger-sale mechanism (atomic: token-release and KAS-payment
      both complete or neither). The covenant guarantees token MATH but not payment; a swap covenant
      closes that. Future project, same stack, embodies the prime directive at the trade layer.
- [ ] Deliberate "make BitPhoque public" pass on GitHub (repo name + README). Secret-sweep first.
- [ ] Mainnet: clear EVERY item in `bitphoque_mainnet_checklist.md` (air-gapped key, V1 compiler,
      detached signing, rejection tests, second review). The TN10 key NEVER becomes the mainnet key.

### TN10 ground facts
- Node: PsychoNode at 192.168.0.206 — wRPC borsh 17210, gRPC 16210. Release tn10-toc3.
- BitPhoque P2PK funding addr (now spent/empty post-genesis):
  `kaspatest:qrwzyuc8lkvdsxgww53n7rsjetsqhh6p9hmy8vrvqpdun5r7ysevufmglhgl4`
- Fee floor learned: ~167,100 sompi; use 200,000.
- rusty-kaspa clone at C:\kaspa-tn12\rusty-kaspa (tag tn10-toc3); silverscript at C:\kaspa-tn12\silverscript.

---

## Reference (the thing we adapted)
- `kcc20.sil` — the token contract (copied verbatim as bitphoque.sil).
- `kcc20-minter.sil` — the minter contract we deliberately DO NOT use.
- `kcc20_tests.rs` — the executable spec: split/merge/transfer/mint/burn + rejection tests. THE
  blueprint for the TRANSFER harness. Shows leader/delegate sigscript construction, owner signing,
  and the handoff/split flow. Lines ~156-313 = the core token-lifecycle test.
- OracleRep `oracle_genesis_establish.rs` — the genesis harness model (already adapted).
