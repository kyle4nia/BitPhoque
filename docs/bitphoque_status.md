# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working
discipline applies: one command at a time, single-line terminal commands, plain-English-before-code,
source-of-truth files committed to git, dry-run before broadcast, verify-by-reading-source for
anything that enforces, local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`)
and its OWN git repo — separate from oracle-protocol. The end-of-session git checkpoint ritual now
covers TWO trees._

_Last updated: **GENESIS LIVE; TRANSFER HARNESS BUILT, BLOCKED ON A REAL LIMIT.** Genesis covenant
UTXO is live on-chain (tx f8d6d45a..., covenant_id 69262e04...). The transfer (handoff) harness
compiles, runs, connects, finds the live genesis UTXO, and signs — but FAILS at the final step:
revealing the 1540-byte redeem script in the spending sigscript exceeds Kaspa's 520-byte max push
element (`ElementExceedsMaxSize(1540, 520)`). Resume point = solve the script-size limit (see below)._

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
- **real redeem script size:** **1540 bytes** (this number is now the CENTRAL PROBLEM — see below).
- **owner key:** TN10 PRACTICE KEY ONLY, permanently. Held in `C:\bitphoque\bitphoque_owner.key`
  (gitignored, never committed — verified clean). Was pasted in chat once during the practice run;
  that BURNS it by mainnet standards, which is fine for a throwaway testnet key. The mainnet owner
  key must be born air-gapped per the checklist and NEVER does this.

NOTE: genesis put the WHOLE funding balance into the single genesis covenant UTXO (full-minus-fee).
The 42,069 phoques live in that covenant UTXO at the token P2SH; "holding" them = controlling the
owner key. There is no wallet that shows a balance — query the P2SH on-chain to see them.

---

## THE TRANSFER HARNESS + THE 520-BYTE WALL (current frontier)

`rust-examples\bitphoque_transfer.rs` (git 6cfb604) — a HANDOFF: spends the genesis covenant UTXO,
moves all 42_069 phoques to a fresh recipient key via the leader entrypoint `__leader_transfer`.

**What WORKS (proven in dry-run):** compiles clean; generates a recipient keypair; connects to the
node; derives the token P2SH via `extract_script_pub_key_address(&current_p2sh, Prefix::Testnet)` and
**finds the live genesis covenant UTXO at it** (confirms our address derivation matches where the
coins actually live); recompiles both current+recipient contracts in-process; signs input 0 with the
current owner key (schnorr, SIG_HASH_ALL, +type byte); builds the leader sigscript args via
`build_sig_script_for_covenant_decl("transfer", [newStates, sigs, witnesses], {is_leader:true})`.
The ENTIRE pipeline is correct up to the last line.

**What FAILS:** the final step appends the redeem script to the sigscript (P2SH spend reveals the
script being satisfied) via `ScriptBuilder::add_data(&script)`. The script is **1540 bytes**; Kaspa
script's max single push element is **520 bytes**. Panic: `ElementExceedsMaxSize(1540, 520)`.

**Why genesis didn't hit this:** genesis PAYS TO the P2SH (commits only a 32-byte hash) — it never
reveals the script. Spending a P2SH REQUIRES revealing the full redeem script, so the limit only
bites on transfer. This is a fundamental P2SH-spend constraint, not a harness bug.

### How the harness is built (Path A — silverscript clone hosts it)
Built INSIDE the silverscript clone as a bin (`silverscript-lang\src\bin\bitphoque_transfer.rs`,
DISPOSABLE copy; canonical is `rust-examples\`). It needs BOTH the silverscript compiler API AND the
kaspa networking crates — only the silverscript workspace has both. The clone is upstream-tracked
(origin/master), so we do NOT commit into it; instead the REBUILD RECIPE is recorded here:
```
# from C:\kaspa-tn12\silverscript, add the networking crates silverscript-lang lacked:
cargo add kaspa-grpc-client --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-notify      --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-rpc-core    --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-addresses   --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add kaspa-hashes      --git https://github.com/kaspanet/rusty-kaspa --branch tn12 -p silverscript-lang
cargo add tokio --features macros,rt-multi-thread -p silverscript-lang
# then copy canonical harness into the clone bin dir and build:
copy C:\bitphoque\rust-examples\bitphoque_transfer.rs C:\kaspa-tn12\silverscript\silverscript-lang\src\bin\
cargo build -p silverscript-lang --bin bitphoque_transfer
cargo run   -p silverscript-lang --bin bitphoque_transfer        # dry-run
```
(Rebuilding the kaspa networking tree from git is a multi-minute first compile. NOTE: `cargo add`
bumped all kaspa crates from commit 42b734f1 -> ab4c51af; if anything behaves oddly, suspect that.)

### Decisions baked into the harness
- HANDOFF first (1-in, 1-out, full amount) — simplest transfer; split/merge are variations.
- Path A chosen over porting `push_typed_sigscript_arg` into rothschild: porting means hand-forking
  live compiler code (fragile, drifts). Adding networking crates to silverscript is mechanical+known.
  (Reading the encoder source is what revealed porting was the WORSE path — verify-first paid off.)

---

## RESUME POINT — solve the 520-byte redeem-script reveal (the wall)

The transfer is blocked ONLY by: a 1540-byte redeem script cannot be revealed as one 520-byte-max
push element when spending the P2SH. Everything else in the transfer pipeline is proven working.
Investigate, in roughly this order:

- [ ] **Read how `kcc20_tests.rs` spends successfully.** The tests run handoff/split/merge and PASS.
      Either (a) `execute_input_with_covenants` runs the engine in-process and bypasses the standard
      push-size check, meaning real on-chain spends face a limit the tests don't — OR (b) there is a
      large-script reveal mechanism we haven't used. READ `common.rs` `push_redeem_script` +
      `execute_input_with_covenants` and the txscript engine's element-size enforcement. Verify which.
- [ ] **Check whether Kaspa P2SH allows multi-push / chunked redeem reveal** (push the script in
      <=520B chunks + OpCat-style reassembly, or a covenant-specific reveal path). Read txscript
      `pay_to_script_hash_script` and the P2SH spend verification in the engine.
- [ ] **If the script genuinely must fit in 520B for on-chain spend:** the contract has to SHRINK.
      The 1540B comes from maxCovIns/Outs=2 fanout + all THREE ownership modes (pubkey/scripthash/
      covenantid) + minter plumbing. A minimal BitPhoque (pubkey-only ownership, 1x1 or smaller
      fanout, minter code stripped) would compile much smaller. This trades features for spendability.
      Would require a contract edit + re-baseline + NEW genesis (covenant_id changes) — i.e. start the
      on-chain story over with a leaner contract. Weigh against keeping 2x2/3-mode.
- [ ] Once spendable: finish handoff dry-run -> broadcast; then REJECTION TESTS (conservation break
      MUST fail; wrong/missing sig MUST fail) — accept-AND-reject, the only real proof.
- [ ] New `.rs` => always-save to `C:\bitphoque\rust-examples\`.

### Later / optional
- [ ] Cosmetic: rename `contract KCC20` -> `BitPhoque` in the .sil (changes bytes; re-baseline).
- [ ] SWAP COVENANT — the trustless stranger-sale mechanism (atomic: token-release and KAS-payment
      both complete or neither). The covenant guarantees token MATH but not payment; a swap covenant
      closes that. Future project, same stack, embodies the prime directive at the trade layer.
- [ ] Deliberate "make BitPhoque public" pass on GitHub (repo name + README). Secret-sweep first.
- [ ] Mainnet: clear EVERY item in `bitphoque_mainnet_checklist.md` (air-gapped key, V1 compiler,
      detached signing, rejection tests, second review). The TN10 key NEVER becomes the mainnet key.
      NOTE: the 520-byte limit is ALSO a mainnet gate — whatever solves it on TN10 must hold on mainnet.

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
  modes sit unused. (NOTE: those unused modes inflate the script — relevant to the 520-byte problem.)
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
- **real redeem script: 1540 bytes** (confirmed at genesis + transfer runtime). THE 520-byte problem.
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
    bitphoque_transfer.rs              <- transfer/handoff harness (blocked on 520B reveal)
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
  blueprint for the TRANSFER harness. Shows leader/delegate sigscript construction, owner signing,
  and the handoff/split flow. Lines ~156-313 = the core token-lifecycle test. **Re-read this to learn
  how on-chain spends get past the 520-byte reveal limit (or whether tests sidestep it in-process).**
- `common.rs` — `push_redeem_script`, `covenant_decl_sigscript`, `execute_input_with_covenants`.
- `silverscript-lang/src/compiler/mod.rs` — public API used by the harness: `compile_contract`,
  `struct_object`, `CompileOptions`, `CovenantDeclCallOptions`, `build_sig_script_for_covenant_decl`.
- OracleRep `oracle_genesis_establish.rs` — the genesis harness model (already adapted).
