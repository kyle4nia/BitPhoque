# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working
discipline applies: one command at a time, single-line terminal commands, plain-English-before-code,
source-of-truth files committed to git, dry-run before broadcast, verify-by-reading-source for
anything that enforces, local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`)
and its OWN git repo — separate from oracle-protocol. The end-of-session git checkpoint ritual now
covers TWO trees._

_Last updated: **TRANSFER IS LIVE ON TN10. (Was: proven locally, — accept AND reject. now BROADCAST + ACCEPTED on-chain, tx 907f7383...; genesis P2SH now EMPTY per inspect tool; 42069 phoques moved to recipient P2SH. Full mint+transfer lifecycle proven.)** Genesis is live
(tx f8d6d45a..., cov_id 69262e04...). The 520-byte wall was a builder-default, NOT a consensus rule
(covenant scripts get a 1,000,000-byte element cap); fixed with `ScriptBuilder::with_flags`. The
handoff now builds, PASSES local VM verify, and the rejection tests prove the gate (accepts valid,
rejects inflation AND wrong-sig). Resume point = broadcast the verified handoff with `--go`._

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
- **genesis covenant UTXO outpoint (the TRANSFER input):** `f8d6d45a...0875:0`
- **spent funding outpoint:** `9f00751f3030b70e79776e54ded48397257915696200b37884ffd3c8ca622823:0`
- **real redeem script size:** **1540 bytes** (fine on-chain — covenant element cap is 1,000,000, see below).
- **owner key:** TN10 PRACTICE KEY ONLY, permanently. Held in `C:\bitphoque\bitphoque_owner.key`
  (gitignored, never committed — verified clean). Was pasted in chat once during the practice run;
  that BURNS it by mainnet standards, which is fine for a throwaway testnet key. The mainnet owner
  key must be born air-gapped per the checklist and NEVER does this.

NOTE: genesis put the WHOLE funding balance into the single genesis covenant UTXO (full-minus-fee).
The 42,069 phoques live in that covenant UTXO at the token P2SH; "holding" them = controlling the
owner key. There is no wallet that shows a balance — query the P2SH on-chain to see them.

---

## TRANSFER — PROVEN LOCALLY (accept AND reject). THE 520 WALL: SOLVED.

`rust-examples\bitphoque_transfer.rs` (git 30061d5) — a HANDOFF: spends the genesis covenant UTXO,
moves all 42_069 phoques to a fresh recipient key via the leader entrypoint `__leader_transfer`.

### The 520-byte wall was NOT real — it was a builder default [RESOLVED]
The transfer first panicked `ElementExceedsMaxSize(1540, 520)` pushing the redeem script. Reading the
txscript engine source (`crypto/txscript/src/lib.rs:80-81,143` + `data_stack.rs:331`) revealed:
- `MAX_SCRIPT_ELEMENT_SIZE_PRE_TOCCATA = 520`; `MAX_SCRIPT_ELEMENT_SIZE_POST_TOCCATA = 1_000_000`.
- `max_script_element_size(covenants_enabled)` returns **1,000,000 when covenants are enabled**.
- The consensus engine therefore ACCEPTS our 1540-byte script. The 520 cap was only the conservative
  default of `ScriptBuilder::new()` (which sets `covenants_enabled:false`).
- **THE FIX (one line):** build the redeem push with
  `ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() })`.
  `script_builder.rs:75` `with_flags` + `validate_data_push` (line 192) lifts the cap to 1M.
- **CONSEQUENCE:** NO contract surgery, NO regenesis, NO chunked reveal. The original 1540-byte
  contract + live genesis are fine. The 1x1/slim shrink experiments (git 961959b) are ABANDONED,
  kept only as evidence (1x1 fanout=920B, slim pubkey-only=799B — both still >520, both moot).
  Verify-before-gutting paid off hard: we nearly regenesis'd a leaner contract to beat a non-limit.

### Local VM verify — INTEGRATED and PASSING
The harness now runs the built+signed handoff through the in-process `TxScriptEngine`
(`covenants_enabled`, `CovenantsContext`) BEFORE broadcast — the `local_vm_verify` fn, modeled on
`kcc20_tests.rs::execute_input_with_covenants`. It runs every invocation (even dry-run) and `--go` is
HARD-GATED behind a pass. Dry-run result: **LOCAL VM VERIFY: PASS — the covenant accepts the spend.**

### Rejection tests — GATE PROVEN (accept AND reject)
`rust-examples\bitphoque_reject_tests.rs` (git 30061d5) — self-contained, NO networking; fabricates
the spend locally (like the kcc20 tests, fabricated cov_id `COV_A` on both input UTXO and output
binding) and runs three cases through the same VM:
- **CONTROL** (1000->1000, signed by owner): **PASS** (expected). Valid handoff accepted.
- **REJECT 1** (1000->50000 inflation, signed by owner): **FAIL** (expected). Conservation is REAL.
- **REJECT 2** (1000->1000, signed by WRONG key): **FAIL** (expected). Signature check is REAL.
- **RESULT: GATE PROVEN.** Enforcement is not a no-op (the `checkDataSig`-style trap is ruled out).
KEY LESSON from building this: the output's `script_public_key` MUST be the RECIPIENT's compiled
P2SH (encoding new owner + new amount), not the genesis P2SH — the covenant validates output state
against the new-state array, and a mismatch yields `VerifyError`. (First control run failed on exactly
this; fixed by compiling the recipient contract per output.)

### Mechanics (all read from kcc20_tests.rs + silverscript-lang public API)
recompile contract with NEW owner -> recipient P2SH; find live genesis cov UTXO at current token P2SH
via `extract_script_pub_key_address`; build unsigned tx (out bound `CovenantBinding{authorizing_input:0,
covenant_id}`); sign input 0 (schnorr, SIG_HASH_ALL, +type byte) via `calc_schnorr_signature_hash`;
leader sigscript via `build_sig_script_for_covenant_decl("transfer",[newStates,sigs,witnesses],
{is_leader:true})` + `with_flags` redeem push. Dry-run default, `--go` broadcasts.

### How the harnesses are built (Path A — silverscript clone hosts them)
Both transfer + reject-tests are bins in the silverscript clone (`silverscript-lang\src\bin\`,
DISPOSABLE copies; canonical in `rust-examples\`). They need BOTH the silverscript compiler API AND
kaspa crates — only the silverscript workspace has both. The clone is upstream-tracked (origin/master),
so we do NOT commit into it; the REBUILD RECIPE (run from `C:\kaspa-tn12\silverscript`):
```
cargo add kaspa-grpc-client --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-notify      --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-rpc-core    --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-addresses   --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-hashes      --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add tokio --features macros,rt-multi-thread -p silverscript-lang
# (reject-tests also uses kaspa-txscript-errors, already transitively present)
copy C:\bitphoque\rust-examples\bitphoque_transfer.rs     C:\kaspa-tn12\silverscript\silverscript-lang\src\bin\
copy C:\bitphoque\rust-examples\bitphoque_reject_tests.rs C:\kaspa-tn12\silverscript\silverscript-lang\src\bin\
cargo build -p silverscript-lang --bin bitphoque_transfer
cargo build -p silverscript-lang --bin bitphoque_reject_tests
cargo run   -p silverscript-lang --bin bitphoque_reject_tests   # gate proof, no node needed
cargo run   -p silverscript-lang --bin bitphoque_transfer       # dry-run incl. local VM verify
```
(First compile of the kaspa networking tree is multi-minute. `cargo add` bumped kaspa crates
42b734f1 -> ab4c51af; if anything behaves oddly, suspect that.)

---

## HANDOFF: LIVE ON TN10 [DONE]

- handoff tx (ACCEPTED): 907f738332268109aea5bacc0670fb59c20af76f17e55c173ca83aa9416a4da1
- genesis cov UTXO f8d6d45a...0875:0 SPENT; genesis token P2SH now EMPTY (inspect tool confirms).
- recipient (now owns all 42069): pubkey 6687942178295b30101e84115f8caf56dbcc0a20008c352587ce4e76da3718b0
  privkey [REDACTED-TN10-KEY]  (TN10 practice key; throwaway).
- THREE consensus rules learned at broadcast (all now handled in the harness):
  1. v1/Toccata txs use ComputeBudget for input mass, NOT SigopCount (rejected: SigopCountInV1).
     sign.rs uses ComputeBudget(10) for v1; we derive ours (see #2).
  2. The covenant spend costs 111,261 script units. Budget is sized via the consensus helper
     ComputeBudget::checked_covering_script_units(ScriptUnits(111261)) yields ComputeBudget(11).
     (const COVENANT_SCRIPT_UNITS in the harness; update it if the contract changes.)
  3. Transfer fee floor ~378,600 (we use 400,000) -- MUCH higher than genesis (~167k) because the
     spending tx REVEALS the full 1540-byte redeem script, inflating tx mass.
- Local VM verify PASSED before broadcast; node ACCEPTED. Local VM and consensus agree.

## NEXT RESUME POINT — broadcast the verified handoff, then re-confirm on-chain

The handoff is DONE (above). Remaining transfer work:

- [x] **Broadcast the handoff** -- DONE (tx 907f7383, ACCEPTED). [x] Re-confirmed: genesis P2SH EMPTY.
- [ ] **Generalize the handoff to a SPECIFIED recipient** (not a random throwaway key) so transfers go
      to a chosen owner -- the obvious next harness improvement.
- [ ] **Add split (1->2) and merge (2->1)** variants; the 2x2 fanout already supports them.
- [ ] **Update inspect tool** to take an address/contract so it can confirm the recipient P2SH balance.
- [ ] (superseded) old broadcast notes: `cargo run -p silverscript-lang --bin bitphoque_transfer -- --go`.
      It re-verifies locally (gate), then submits. NOTE: each run generates a FRESH recipient key and
      prints its privkey — SAVE that privkey, it owns the phoques after the handoff. The genesis UTXO
      (`f8d6d45a...0875:0`) is spent by this; a new covenant UTXO appears at the recipient's P2SH.
- [ ] **Re-confirm with the inspect tool:** after broadcast, the genesis P2SH should be EMPTY and the
      phoques should live at the recipient's P2SH. (Inspect tool currently hardcodes the genesis
      compiled script; to check the recipient P2SH, point it at the recipient's compiled contract, or
      generalize it to take an address.)
- [ ] New `.rs` => always-save to `C:\bitphoque\rust-examples\`.

### Later / optional
- [ ] Generalize the handoff harness: take a SPECIFIED recipient pubkey (not a random throwaway) so
      transfers go to a chosen owner; add split (1->2) and merge (2->1) variants (the 2x2 fanout
      already supports them).
- [ ] Cosmetic: rename `contract KCC20` -> `BitPhoque` in the .sil (changes bytes; re-baseline + regenesis).
- [ ] SWAP COVENANT — the trustless stranger-sale mechanism (atomic: token-release and KAS-payment
      both complete or neither). The covenant guarantees token MATH but not payment; a swap covenant
      closes that. Future project, same stack, embodies the prime directive at the trade layer.
- [ ] Deliberate "make BitPhoque public" pass on GitHub (repo name + README). Secret-sweep first.
- [ ] Mainnet: clear EVERY item in `bitphoque_mainnet_checklist.md` (air-gapped key, V1 compiler,
      detached signing, rejection tests, second review). The TN10 key NEVER becomes the mainnet key.

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
  modes sit unused (we kept them: with the 520 non-issue resolved, there's no size pressure to strip).
- **Supply:** FIXED at genesis, NO minter. **42,069 phoques** (TN10 placeholder — may pick a cleverer
  number before mainnet; it's just a genesis ctor arg, fully reversible). The whole supply is minted
  once into a genesis UTXO and can only ever be split / merged / transferred, never increased.
- **Self-erecting property:** satisfied by construction. With NO minter covenant deployed and every
  token UTXO genesis'd `isMinter=false`, conservation is ALWAYS enforced and supply is mathematically
  fixed. No privileged operator can inflate or freeze. Every holder is sovereign over their own UTXOs
  via their own key. **NOW PROVEN, not just asserted** (rejection tests: inflation + wrong-sig FAIL).

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
- **real redeem script: 1540 bytes** (fine on-chain; covenant element cap is 1,000,000).
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
    bitphoque.sil                      <- the contract (KCC20 verbatim, canonical, DEPLOYED)
    bitphoque_ctor.json                <- placeholder ctor (32-zero pubkey, baseline)
    bitphoque_ctor_real.json           <- REAL ctor (owner pubkey baked in)
    bitphoque_compiled_baseline.json   <- frozen known-good baseline bytecode
    bitphoque_compiled_real.json       <- REAL compiled contract (harness reads script from here)
    bitphoque_slim.sil                 <- ABANDONED experiment (pubkey-only, 520-investigation)
    bitphoque_ctor_1x1.json            <- ABANDONED experiment (1x1 fanout ctor)
    bitphoque_compiled_1x1.json        <- ABANDONED experiment (1x1 compile, 920B)
    bitphoque_slim_1x1.json            <- ABANDONED experiment (slim+1x1 compile, 799B)
  rust-examples\
    bitphoque_keygen.rs                <- owner keypair generator
    bitphoque_genesis_establish.rs     <- genesis harness (LIVE genesis sent with this)
    bitphoque_transfer.rs              <- transfer/handoff harness (builds, VM-verify PASS, ready to --go)
    bitphoque_reject_tests.rs          <- gate proof (accept + reject, no node needed)
    bitphoque_inspect.rs               <- read-only on-chain confirm tool
  docs\
    bitphoque_status.md                <- this file
    bitphoque_mainnet_checklist.md     <- the pre-mainnet GATE
    operating_baseline.md              <- shared portable baseline (identical copy in oracle tree)
```
Own git repo (branch `master`). NOT yet pushed to GitHub — later deliberate "make it public" pass
(clean repo name + README explaining the phoque joke + fixed-supply/no-coordinator design).

---

## TN10 ground facts
- Node: PsychoNode at 192.168.0.206 — wRPC borsh 17210, gRPC 16210. Release tn10-toc3.
- BitPhoque P2PK funding addr (now spent/empty post-genesis):
  `kaspatest:qrwzyuc8lkvdsxgww53n7rsjetsqhh6p9hmy8vrvqpdun5r7ysevufmglhgl4`
- Genesis covenant UTXO (the transfer input): `f8d6d45a...0875:0`, value 9_999_999_800_000 sompi.
- Token P2SH address (where BitPhoque lives on-chain): `kaspatest:prrejfkltxlctd0htkma7p50l45tc7fy242vf4g475tc2y0k5rlggne36eqz6`
- Fee floor learned: ~167,100 sompi; use 200,000.
- rusty-kaspa clone at C:\kaspa-tn12\rusty-kaspa (tag tn10-toc3); silverscript at C:\kaspa-tn12\silverscript.

---

## Reference (the thing we adapted)
- `kcc20.sil` — the token contract (copied verbatim as bitphoque.sil).
- `kcc20-minter.sil` — the minter contract we deliberately DO NOT use.
- `kcc20_tests.rs` — the executable spec: split/merge/transfer/mint/burn + rejection tests. THE
  blueprint for the TRANSFER + reject-test harnesses. Shows leader/delegate sigscript construction,
  owner signing, the handoff/split flow, and the fabricated-UTXO local-test pattern. Lines ~156-313.
- `common.rs` — `push_redeem_script`, `covenant_decl_sigscript`, `execute_input_with_covenants`
  (the model for `local_vm_verify`).
- `crypto/txscript/src/lib.rs` + `script_builder.rs` — where the 520-vs-1,000,000 element cap and
  `ScriptBuilder::with_flags` live (the 520-wall resolution).
- `silverscript-lang/src/compiler/mod.rs` — public API used by the harnesses: `compile_contract`,
  `struct_object`, `CompileOptions`, `CovenantDeclCallOptions`, `build_sig_script_for_covenant_decl`.
- OracleRep `oracle_genesis_establish.rs` — the genesis harness model (already adapted).
