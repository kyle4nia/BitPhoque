# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working discipline applies: one
command at a time, single-line terminal commands, plain-English-before-code, source-of-truth files
committed to git, dry-run before broadcast, verify-by-reading-source for anything that enforces,
local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`) and its OWN git repo —
separate from oracle-protocol. The end-of-session git checkpoint ritual covers TWO trees._

_Last updated: **FIXED-PRICE SALE PROVEN END-TO-END ON-CHAIN (TN10).** Full lifecycle now demonstrated
on a real node: MINT (genesis) -> TRANSFER (handoff) -> SALE (list + claim). The sale was executed on a
SECOND, sale-enabled deployment (v2 contract with per-UTXO forSale/price): a buyer claimed a listed UTXO
by paying the seller, receiving the phoques with NO owner signature on the covenant input (payment-
authorized). Resume point = generalize the sale beyond single-input/output (2x2 fanout), then the
pre-mainnet checklist._

---

## CURRENT STATE (at a glance)

- **Original token (v1 contract, fungible):** LIVE. Genesis + transfer proven on-chain. Transfer reject-
  tests prove the gate (accept valid, reject inflation + wrong-sig).
- **Sale mechanism (v2 contract, per-UTXO forSale/price):** the trustless fixed-price sale. PROVEN at the
  VM level (claim + listing, all cheats rejected) AND end-to-end ON-CHAIN (genesis -> list -> claim).
- **Two spend modes in v2:** LISTING (owner-signed, flips a UTXO to forSale=true at a price) and CLAIM
  (payment-authorized, NO owner sig — a buyer pays >= price to the seller and receives the phoques).
- **Auction:** recorded as a separate FUTURE build (see swap design doc) — stateful/temporal, own architecture.
- **Next:** generalize the sale beyond single-input/output (2x2 fanout + multi-claim output-matching);
  then the pre-mainnet checklist. Design + open items live in `bitphoque_swap_covenant_design.md`.

---

## What this is (one sentence)

BitPhoque is a covenant-based, native-Layer-1 fungible token on Kaspa TN10 — a practice run for a
mainnet launch — built on the SAME stack as the Oracle Protocol (silverscript covenants, the
`binding=cov` N-to-M plumbing), deliberately NOT the indexer-based KRC-20 path. It is a principled
novelty token: phonetic play on "out of f**ks to give," seal (phoque) mascot, fixed supply, no
trusted coordinator. Site: https://www.outlawpolitics.org/BitPhoque/ — Family Guy S8E18
"Quagmire's Dad" is the cultural reference (Chris's French homework: "phoque" = seal).

---

## ON-CHAIN HISTORY (TN10) — what's live, in order

### 1. GENESIS (original token) — LIVE
- tx: `f8d6d45a37f6049872bb12d55359a939e6677e6848ad6135a9e1fd2cbbb00875`
- covenant_id: `69262e04b63d1ea1008fd33f8aff38c08780056b40111d0db01a306ef11a766c`
- owner pubkey (x-only): `dc227307fd98d8190e75233f0e12cae00bdf412df643b06c005bc9d07e2432ce`
- genesis covenant value: 9,999,999,800,000 sompi (100k TKAS minus 200k fee)
- real redeem script: 1540 bytes (fine on-chain — covenant element cap is 1,000,000).
- Token P2SH: `kaspatest:prrejfkltxlctd0htkma7p50l45tc7fy242vf4g475tc2y0k5rlggne36eqz6`
- The whole supply (42,069) minted into one genesis UTXO; "holding" = controlling the owner key.

### 2. TRANSFER (handoff) — LIVE
- handoff tx (ACCEPTED): `907f738332268109aea5bacc0670fb59c20af76f17e55c173ca83aa9416a4da1`
- genesis cov UTXO `f8d6d45a...0875:0` SPENT; genesis token P2SH now EMPTY (inspect tool confirmed).
- recipient (owns all 42069 of the ORIGINAL token): pubkey
  `6687942178295b30101e84115f8caf56dbcc0a20008c352587ce4e76da3718b0`
  privkey `[REDACTED-TN10-KEY] ` (TN10 practice; throwaway).
- harness: `rust-examples/bitphoque_transfer.rs`. Local VM verify gates `--go`.

### 3. SALE (v2 sale-enabled token) — LIVE, full lifecycle proven on-chain
A SECOND deployment (v2 contract) to prove the fixed-price sale. Separate from the original token.
- v2 contract: `covenants/bitphoque_sale_v2.sil` (checkSaleState listing/claim separation). Script 1969 bytes.
- v2 covenant_id: `9a30cef719c6ac08eaf94f701dba43136f3a6837b4d24c419610507250367bf3`
- v2 token OWNER (the SELLER): `50a1553911677db039333592faf1c0757a4e18999fd9be68da0f1b1ab60cb11b`
  (privkey in gitignored `bitphoque_v2_owner.key`). Payout addr: `kaspatest:qpg2z4fez9nhmvpexv6e97h3cp6h5nscnx0an0ngmg83kx4kpjc3k6s5w25c5`
- BUYER key: `261cbf44ce71a1501fe7fc2d6191ccd657c0fd170297b2b871533ced62c54420`
  (privkey in gitignored `bitphoque_buyer.key`). Faucet-funded separately (genuinely distinct parties).
- **(3a) GENESIS v2** tx: `b1086a59c163c4fbd8f9c333fc419f517c695e41f8989ddb23d7e945fbcdc6e2` (42069, not-for-sale).
  harness `rust-examples/bitphoque_sale_genesis.rs` (reads `bitphoque_sale_compiled.json`).
- **(3b) LISTING** tx (ACCEPTED): `cad1220f61494e70f6a15b3316ea12d4f218c0929a36bd0fc86c739d68835ea0`
  Owner-signed; flipped the genesis UTXO to forSale=true at price 500,000 sompi. New for-sale UTXO `cad1220f...:0`.
  harness `rust-examples/bitphoque_sale_list.rs`.
- **(3c) CLAIM** tx (ACCEPTED): `592a0d5b54469e78d4b43af4298a6ec063b78cf3dd7d1fc708af688920934eb7`
  Buyer paid 100,000,000 sompi to the seller and received the 42069 phoques — NO owner sig on the covenant
  input (payment-authorized). Phoques now in a new covenant UTXO owned by the BUYER key, not-for-sale.
  harness `rust-examples/bitphoque_sale_claim.rs` (2-input: unsigned covenant input 0 + buyer-signed plain input 1).

---

## CONSENSUS RULES LEARNED (the broadcast lessons — all handled in the harnesses)

From the TRANSFER (spends a covenant UTXO, single input):
1. v1/Toccata txs use ComputeBudget for input mass, NOT SigopCount (reject: SigopCountInV1). sign.rs uses
   ComputeBudget(10) for v1.
2. The v1 covenant spend costs 111,261 script units. Budget sized via the consensus helper
   `ComputeBudget::checked_covering_script_units(ScriptUnits(N))` -> ComputeBudget(11). (const COVENANT_SCRIPT_UNITS.)
3. Transfer fee floor ~378,600 (use 400,000) — much higher than genesis (~167k) because spending REVEALS the
   1540-byte redeem script, inflating tx mass.

From the CLAIM (2-input: covenant + plain buyer funding, 3 outputs):
4. v1 needs ComputeBudget on EVERY input — including the plain P2PK buyer input (SigopCount rejected wholesale in v1).
5. A plain P2PK input costs ~100,000 script units in a covenant tx (NOT cheap; budget with ComputeBudget(11)).
6. KIP-9 STORAGE MASS penalizes dust outputs: storage mass = max(0, C*(|O|/H(O) - |I|/A(I))); the harmonic term
   divides C by output amount, so a tiny payment output (500k sompi) spiked it to 2,000,000 (limit 500,000).
   FIX: pay a non-dust amount. The covenant checks `payment >= price`, so we paid 100M (>> 500k listed price)
   to dodge the dust penalty WITHOUT re-listing. (calc_storage_mass: consensus/core/src/mass/mod.rs:437.)
7. Claim fee floor ~654,600 (use 700,000) for the bigger 2-input, script-revealing tx.

NOTE: genesis spends a PLAIN funding input as a rothschild bin and used SigopCount(1) successfully — the
ComputeBudget-on-all-inputs rule (#4) bit the CLAIM (a v1 covenant tx built via silverscript-lang), not genesis.

---

## THE 520-BYTE WALL — SOLVED (a builder default, not a consensus rule)
The first transfer panicked `ElementExceedsMaxSize(1540, 520)` pushing the redeem script. Reading the txscript
engine source (`crypto/txscript/src/lib.rs:80-81,143` + `data_stack.rs:331`):
- `MAX_SCRIPT_ELEMENT_SIZE_PRE_TOCCATA = 520`; `..._POST_TOCCATA = 1_000_000`.
- `max_script_element_size(covenants_enabled)` returns 1,000,000 when covenants are enabled.
- The 520 cap was only the conservative default of `ScriptBuilder::new()` (covenants_enabled:false).
- FIX (one line): build the redeem push with
  `ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() })`.
- CONSEQUENCE: NO contract surgery, NO regenesis, NO chunked reveal. Verify-before-gutting paid off hard —
  we nearly regenesis'd a leaner contract to beat a non-limit. (Abandoned shrink experiments: git 961959b.)

---

## DESIGN — SETTLED

- **Architecture:** per-UTXO balance (real token, not a single-UTXO ledger). Transfers spend N token UTXOs and
  create M, conservation-enforced.
- **Ownership:** OWNED via per-UTXO signature. Three modes in the contract (`IDENTIFIER_PUBKEY=0x00`,
  `IDENTIFIER_SCRIPT_HASH=0x01`, `IDENTIFIER_COVENANT_ID=0x02`). BitPhoque uses 0x00 (pubkey-owned) — plain
  checkSig, no CSFS needed.
- **Supply:** FIXED at genesis, NO minter. 42,069 phoques (TN10 placeholder; just a genesis ctor arg). Whole
  supply minted once; only split/merge/transfer, never increased.
- **Self-erecting property:** with NO minter covenant deployed and every UTXO genesis'd `isMinter=false`,
  conservation is ALWAYS enforced and supply is mathematically fixed. PROVEN, not asserted (reject-tests).
- **Sale model (v2):** per-UTXO `forSale` (bool) + `price` (int) state. Listing is owner-signed; claiming is
  payment-authorized. checkSaleState allows for-sale OUTPUTS only when NO input is for-sale (a listing);
  forbids them when ANY input is for-sale (a claim — anti-double-claim). DECIDED: per-UTXO (fungible), NOT
  per-phoque ids (which would make it an NFT collection). Full design + open items: `bitphoque_swap_covenant_design.md`.

### Why covenant token, not KRC-20
KRC-20 is inscription-based, interpreted by a trusted off-chain indexer (Kasplex) — token truth depends on a
coordinator. A covenant token enforces semantics (conservation, ownership) at the SCRIPT layer with no
coordinator. NOT blocked on CSFS: an OWNED transfer uses plain OpCheckSig over the spending tx, which works.

---

## THE CONTRACTS

### Original token — `covenants/bitphoque.sil` (DEPLOYED, the live original)
Copied VERBATIM from `kcc20.sil` (faithful-adopt, no fork). Self-identifies as `contract KCC20`.
- Fixed-supply by config: `checkAmounts` (totalIn==totalOut) and `checkMintingTransfer` are gated `if(!isMinter)`;
  genesis with isMinter=false and there is no path to increase supply. Minter contract never deployed.
- Ctor (6 args): `KCC20(byte[32] genesisPk, int genesisAmount, byte genesisIdentifierType, bool genesisIsMinter, int maxCovIns, int maxCovOuts)`.
- Per-UTXO state: ownerIdentifier(byte[32]) + identifierType(byte) + amount(int) + isMinter(bool). state_layout {start:1, len:46}.
- ABI = leader/delegate: `__leader_transfer(newStates, sigs, witnesses)` + `__delegate_transfer()`. Conservation
  auto-wired by the binding=cov macro (readInputState -> prevStates[] -> policy -> validateOutputState per output).
- ctor JSON encodings: byte[32] = array of {"kind":"byte","data":N}; bare byte = {"kind":"byte","data":N};
  bool = {"kind":"bool","data":false}; int = {"kind":"int","data":N}.

### Sale token — `covenants/bitphoque_sale_v2.sil` (DEPLOYED as the v2 sale token)
Extends the token with per-UTXO forSale/price + the sale-claim path. Ctor (8 args): adds `bool genesisForSale,
int genesisPrice` after isMinter. state_layout {start:1, len:57}. Script 1969 bytes.
- `checkAuth`: per-input — not-for-sale needs owner sig; for-sale needs tx.outputs[witnesses[i]] to pay >= price
  to the seller's P2PK lock (witnesses[i] = payment output index).
- `checkSaleState`: if ANY input is for-sale (a claim), forbid for-sale outputs (anti-double-claim); else (a
  listing) allow them. (v1's checkSaleReset was over-strict — forbade ALL for-sale outputs, making listing
  impossible; v2 fixed it. History: bitphoque_sale_v1.sil kept as the superseded version.)
- Compiled for genesis with the v2 owner pubkey: `covenants/bitphoque_sale_compiled.json` + `bitphoque_sale_genesis_ctor.json`.

### Compile command (from silverscript dir; source-of-truth .sil is in bitphoque)
```
cargo run --bin silverc -- C:\bitphoque\covenants\<file>.sil --ctor C:\bitphoque\covenants\<ctor>.json -c
```
Capture clean JSON: append ` > <path>.json 2>nul`.

---

## VM-LEVEL PROOFS (reject-tests — accept AND reject, no node needed)

- **Transfer gate** (`rust-examples/bitphoque_reject_tests.rs`): CONTROL passes; inflation FAILS; wrong-sig FAILS.
  Enforcement is real (the checkDataSig no-op trap ruled out). KEY LESSON: the output's script_public_key MUST be
  the RECIPIENT's compiled P2SH (encoding new owner + amount), not the genesis P2SH — else VerifyError.
- **Sale + listing gate** (`rust-examples/bitphoque_sale_reject_tests.rs`, against sale_v2): CLAIM path — control
  passes; 6 cheats rejected (underpay / wrong-payee / no-payment / claim-unlisted / stays-for-sale / output-match-abuse).
  LISTING path — control passes (listing works); no-sig and wrong-sig rejected (listing REQUIRES owner sig).
- These are SINGLE-input/output proofs. Fanout (2x2) generalization is the next open item.

---

## HARNESSES & HOW THEY'RE BUILT

Networking harnesses (transfer, sale_list, sale_claim, reject-tests) are bins in the silverscript clone
(`silverscript-lang\src\bin\`, DISPOSABLE copies; CANONICAL in `C:\bitphoque\rust-examples\`). They need BOTH the
silverscript compiler API AND kaspa crates — only the silverscript workspace has both. The clone is upstream-tracked,
so do NOT commit into it. Genesis-class harnesses (genesis_establish, sale_genesis, inspect, keygen) are rothschild
bins (`C:\kaspa-tn12\rusty-kaspa\rothschild\src\bin\`). Every new .rs is canonical-saved to rust-examples IN ADDITION
to the disposable build location.

Rebuild recipe (from `C:\kaspa-tn12\silverscript`): `cargo add kaspa-grpc-client/notify/rpc-core/addresses/hashes
--git rusty-kaspa --branch tn12 -p silverscript-lang`; `cargo add tokio --features macros,rt-multi-thread`;
copy the .rs into `silverscript-lang\src\bin\`; `cargo build/run -p silverscript-lang --bin <name>`.
Genesis-class: `cargo build/run --release -p rothschild --bin <name>`.

---

## File tree (`C:\bitphoque`)
```
C:\bitphoque\
  .gitignore                           <- ignores *.key (all key files)
  bitphoque_owner.key                  <- original token owner / v2 FUNDING signer (GITIGNORED)
  bitphoque_v2_owner.key               <- v2 sale token owner = SELLER (GITIGNORED)
  bitphoque_buyer.key                  <- v2 sale BUYER (GITIGNORED)
  covenants\
    bitphoque.sil                      <- original token contract (KCC20 verbatim, DEPLOYED)
    bitphoque_ctor.json / _ctor_real.json / _compiled_*.json  <- original ctors + compiled
    bitphoque_sale_v1.sil              <- sale contract v1 (SUPERSEDED — listing was impossible)
    bitphoque_sale_v2.sil              <- sale contract v2 (DEPLOYED as the v2 sale token)
    bitphoque_sale_ctor.json           <- v2 sale ctor (test/8-arg shape)
    bitphoque_sale_genesis_ctor.json   <- v2 genesis ctor (real owner pubkey baked in)
    bitphoque_sale_compiled.json       <- v2 compiled (sale_genesis harness reads script from here)
    (abandoned 520-investigation experiments: bitphoque_slim.sil, *_1x1.json — kept as evidence)
  rust-examples\
    bitphoque_keygen.rs                <- keypair generator
    bitphoque_genesis_establish.rs     <- original genesis harness
    bitphoque_transfer.rs              <- transfer/handoff harness (LIVE transfer)
    bitphoque_reject_tests.rs          <- original transfer gate proof
    bitphoque_inspect.rs               <- read-only on-chain confirm tool
    bitphoque_sale_v2 ...              <- (contract is in covenants\)
    bitphoque_sale_reject_tests.rs     <- sale+listing gate proof (claim + listing, all cheats)
    bitphoque_sale_genesis.rs          <- v2 genesis harness (LIVE v2 genesis)
    bitphoque_sale_list.rs             <- LISTING broadcast harness (LIVE listing)
    bitphoque_sale_claim.rs            <- CLAIM broadcast harness (LIVE claim)
  docs\
    bitphoque_status.md                <- this file
    bitphoque_swap_covenant_design.md  <- sale/swap design + auction future-build + open items
    bitphoque_mainnet_checklist.md     <- the pre-mainnet GATE
    operating_baseline.md              <- shared portable baseline (copy in oracle tree)
```
Own git repo (branch `master`). NOT yet pushed to GitHub — later deliberate "make it public" pass
(clean repo name + README + secret-sweep first).

---

## TN10 ground facts
- Node: PsychoNode at 192.168.0.206 — wRPC borsh 17210, gRPC 16210. Release tn10-toc3.
- Funding addr (reused for both genesis runs): `kaspatest:qrwzyuc8lkvdsxgww53n7rsjetsqhh6p9hmy8vrvqpdun5r7ysevufmglhgl4`
  (key in bitphoque_owner.key; re-faucet-funded 100k TKAS for the v2 genesis).
- rusty-kaspa clone: C:\kaspa-tn12\rusty-kaspa (tag tn10-toc3); silverscript: C:\kaspa-tn12\silverscript.
- Fee floors learned: genesis ~167k (use 200k); transfer ~378,600 (use 400k); claim ~654,600 (use 700k).

---

## RESUME POINT — 2x2 fanout VM-proven (v3 fix); generalize wider, then mainnet checklist

Fixed-price sale PROVEN end-to-end on-chain at 1x1 (single-input/output) on the v2 contract.
2x2 AND 3x3 FANOUT now PROVEN AT THE VM LEVEL on a NEW v3 contract — and the work FOUND A REAL BUG.

**What 2x2 testing found (and v3 fixes):** at fanout, checkAuth let two for-sale inputs from the
SAME seller name the SAME payment output (witnesses=[2,2], one payment) — a buyer could claim TWO
listings for ONE payment (the "F1 double-claim" hole). v2's per-input "pay this input's seller"
check did NOT enforce that for-sale inputs map to DISTINCT outputs. Cross-seller (F2) was already
rejected by the wrong-payee check; same-seller was the hole.

**The v3 fix (`covenants/bitphoque_sale_v3.sil`):** for-sale inputs must use STRICTLY-INCREASING
payment-output indices (lastSaleWitness accumulator seeded at -1; each for-sale witness must exceed
the previous). No two for-sale inputs can share an output. Smallest provable fix. MAINNET-GRADE
generalization deferred + noted in-contract (option B: per-output price accounting, so one big
output could cover several inputs). v2 stays on disk as the as-deployed/holed version (like v1).

**Proof:** `rust-examples/bitphoque_sale_reject_tests.rs` (extended; 1x1 block intact). All green
against v3: 10x 1x1 (C-CONTROL..C6, L-CONTROL..L2) + 2x2 controls (claim, split-on-list) + 5 fanout
cheats rejected (F1 double-claim, F2 cross-wire, F3 inflate, F4 claim-leaves-listing, F5 mixed-auth).

**IMPORTANT — what is NOT yet done:** v3 has NEVER been broadcast. The LIVE TN10 sale token is still
v2 (the holed version). Fanout proven at 2x2 AND 3x3 at the VM, not NxM (strict-increasing scales; untested
past 2). Remaining, in order:
- [ ] **Re-genesis the sale token on v3** (the live v2 is holed; do not build further sale features on v2).
      Then re-prove list+claim ON-CHAIN at 2x2 (genesis -> list two -> claim both, distinct payments).
- [x] **3x3 fanout PROVEN at VM** (G-CTRL claim/split, G1 triple-share, G2 mid-collision, G3 out-of-order, G4 non-contiguous all behave). NxM inferred (accumulator is N-agnostic) but only 2 and 3 tested. NOTE: strict-increasing rejects DISTINCT-but-UNORDERED witnesses (G3) — safe but stricter than minimal; per-output-accounting mainnet version would relax it.
- [ ] **Split/merge for the sale** (list part of a balance, not the whole supply) — the 2x2 fanout supports it.
- [ ] **Inspect tool**: generalize to take an address/contract so it can confirm arbitrary P2SH balances on-chain.
- [ ] **Original-token owner quirk** (observed, not blocking): the ORIGINAL genesis ctor used a 32-ZERO pubkey as
      genesisPk; how the original transfer satisfied checkSig against a zero owner is unresolved. Does NOT affect
      v2/v3 (real fresh pubkey). Worth understanding later.
- [ ] **AUCTION** — separate future build (stateful/temporal: bid ratchet, escrow, locktime, refunds). Own
      architecture; see the swap design doc's auction section. Fixed-price first (done), auction separately.
- [ ] **Mainnet**: clear EVERY item in `bitphoque_mainnet_checklist.md` (air-gapped key, V1 compiler, detached
      signing, full reject-tests, second review), on v3 (or the per-output-accounting successor). TN10 keys
      NEVER become mainnet keys.
- [ ] **Make BitPhoque public** on GitHub (clean repo name + README explaining the phoque joke + fixed-supply/
      no-coordinator design). Secret-sweep first.

---

## Reference (the things we adapted)
- `kcc20.sil` — token contract (copied verbatim as bitphoque.sil). `kcc20-minter.sil` — deliberately NOT used.
- `covenant_escrow.sil` / `covenant_mecenas.sil` — the output-payment-inspection patterns the sale claim adopts
  (tx.outputs[i].value / .scriptPubKey, new ScriptPubKeyP2PK(party)).
- `kcc20_tests.rs` + `common.rs` — leader/delegate sigscript construction, owner signing, the fabricated-UTXO
  local-test pattern, execute_input_with_covenants (the model for local_vm_verify).
- `crypto/txscript/src/lib.rs` + `script_builder.rs` — the 520-vs-1,000,000 element cap + with_flags.
- `consensus/core/src/mass/mod.rs` — calc_storage_mass (KIP-9, line 437); the ComputeBudget / script-units rules.
- `silverscript-lang/src/compiler/mod.rs` — compile_contract, struct_object, CompileOptions,
  CovenantDeclCallOptions, build_sig_script_for_covenant_decl.
- OracleRep `oracle_genesis_establish.rs` — the genesis harness model.
