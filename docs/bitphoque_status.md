# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working discipline applies: one
command at a time, single-line terminal commands, plain-English-before-code, source-of-truth files
committed to git, dry-run before broadcast, verify-by-reading-source for anything that enforces,
local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`) and its OWN git repo —
separate from oracle-protocol. The end-of-session git checkpoint ritual covers TWO trees._

_Last updated: **COMBINED 2×2 CLAIM PROVEN ON-CHAIN — the F1 fix is now demonstrated on a real node.**
The headline open item is CLOSED: two for-sale UTXOs from the SAME seller were claimed in ONE tx,
forced onto strictly-increasing distinct payment outputs (witnesses=[2,3]), with two buyers each
funding their own input (atomic multi-party settlement). The v2 F1 double-claim (two listings, one
payment) is now structurally impossible AND confirmed on-chain, not just in VM reject-tests. A
realistic 3-holder distribution (buyer1/2/3) is seeded on-chain for future wallet/marketplace work.
Remaining: split/merge generalization, generalized inspect, the wallet/CLI build, mainnet checklist.

---

## CURRENT STATE (at a glance)

- **Original token (v1 contract, fungible):** LIVE. Genesis + transfer proven on-chain. Transfer reject-
  tests prove the gate (accept valid, reject inflation + wrong-sig).
- **Sale mechanism (v2 contract, per-UTXO forSale/price):** the trustless fixed-price sale. PROVEN at the
  VM level (claim + listing, all cheats rejected) AND end-to-end ON-CHAIN (genesis -> list -> claim).
- **Sale mechanism (v3 contract, fanout-safe F1 fix):** LIVE. Full lifecycle proven on-chain at 2×2 —
  split → two listings → two independent claims by two distinct buyers; plus secondary-market resale.
- **Two spend modes:** LISTING (owner-signed, flips a UTXO to forSale=true at a price) and CLAIM
  (payment-authorized, NO owner sig — a buyer pays >= price to the seller and receives the phoques).
- **Auction:** recorded as a separate FUTURE build (see swap design doc) — stateful/temporal, own architecture.
- **Next:** the COMBINED 2×2 claim tx (on-chain F1-fix proof), then generalize wider; then the pre-mainnet
  checklist. Design + open items live in `bitphoque_swap_covenant_design.md`.

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

### 4. SALE v3 (fanout-safe, the F1 fix) — LIVE genesis
First-ever v3 broadcast. v3 fixes the F1 double-claim hole (for-sale inputs must use
strictly-increasing payment-output indices). Supersedes the holed v2 as the live sale token.
- v3 contract: `covenants/bitphoque_sale_v3.sil`. Compiled script 2793 bytes.
- v3 covenant_id: `3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458`
- v3 genesis tx (ACCEPTED + confirmed): `f1e53e2ff1a7c58145b4ecaca0d06b14ff7f1fba2083b95546a861cd87640ddf`
- genesis covenant UTXO `f1e53e2f...:0`, value 3,299,999,800,000 sompi, 42069 phoques, not-for-sale, maxCovIns/Outs=3/3.
- owner = same v2 seller pubkey `50a1...`; funding input `fc91589116b0de5a4f9f019cb3520564ec9d5d014051f41bec8ec0c8c42f2682:0` spent (confirmed empty).
- harness `rust-examples/bitphoque_sale_genesis_v3.rs` (reads `bitphoque_sale_compiled_v3.json`).
- NOTE: funded with 33,000 TKAS (faucet 24hr cap 100k; conserved the rest for list/claim funding).

### 5. SPLIT v3 (1 -> 2, prerequisite for 2x2) — LIVE
The genesis minted all 42069 into one UTXO; a 2x2 sale needs two. Owner-signed 1-input/2-output
transfer, both outputs not-for-sale, same owner. First on-chain MULTI-OUTPUT transfer on v3
(proves conservation across two outputs on a real node).
- split tx (ACCEPTED): `5db053c41fe1865d89fb2c70dfb890b6c5a10bef80e053fcd302d5d47b3aa8e9`
- out 0 `5db053c4...:0` = 21034 phoques, ~16,499.995 TKAS, owner 50a1..., not-for-sale
- out 1 `5db053c4...:1` = 21035 phoques, ~16,499.995 TKAS, owner 50a1..., not-for-sale
- harness `rust-examples/bitphoque_sale_split_v3.rs` (silverscript bin). Caps 3/3 (matches genesis).
- NOTE: each output sits at a DIFFERENT P2SH (amount is a ctor arg -> bakes into script). The list
  harness must compile with the right amount to find each UTXO.

### 6. SALE v3 — 2×2 PRIMARY SALE (two separate claims) — LIVE
The two split UTXOs (21034, 21035) listed and claimed independently by two DISTINCT buyers.
Proves the v3 claim path on a real node and the "sell over time to successive buyers" use case.
- buyer0 = `261cbf44ce71a1501fe7fc2d6191ccd657c0fd170297b2b871533ced62c54420`
  (key `bitphoque_buyer.key`; addr `kaspatest:qqnpe06yeec6z5qlul7z6cv3ent90s8azupf0v4cw9fnemtzc4zzqa8xadms9`)
- buyer1 = `8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936`
  (key `bitphoque_buyer1.key`; addr `kaspatest:qzxq9ss2v7gpz2x6hymhhjeajuujjm5z4skeua3gvdw67clx84ynvxfwgm0j4`)
  GENERATED + faucet-funded this session (faucet tx `3e7b89c1...`, 1000 TKAS; faucet min is 1000).
- **(6a) LISTING 21034** tx: `cebf75b5fafbdad3c38505fdeec4b7db3542bd9b88a448cbab219e6f4caa2443`
  (owner `50a1...` signed; price 500000; for-sale UTXO `cebf75b5...:0`). Spent split `5db053c4...:0`.
- **(6b) LISTING 21035** tx: `55542a993f104dae1d304c95420a8c14f4805a7b6c37909d48a12bbfc361c59f`
  (owner `50a1...` signed; price 500000; for-sale UTXO `55542a99...:0`). Spent split `5db053c4...:1`.
- **(6c) CLAIM 21034 by buyer0** tx: `3265e0635217c37acc32247cbdd34b83072b4e071d9d644b5f4ea6ceedd1f085`
  (buyer0 paid 100M to seller `50a1...`, received 21034; not-for-sale UTXO `3265e063...:0`).
- **(6d) CLAIM 21035 by buyer1** tx: `9ffe84b8a6d56b691c3748b82aaccc35b2b440857ebca650b46c05bba3f6ed6a`
  (buyer1 paid 100M to seller `50a1...`, received 21035; not-for-sale UTXO `9ffe84b8...:0`).
- harnesses: `rust-examples/bitphoque_sale_list_v3.rs` + `rust-examples/bitphoque_sale_claim_v3.rs`.
- NOTE: these are TWO SEPARATE claim txs (one for-sale input each). They prove single claims on v3.
  They do NOT yet exercise the strict-increasing F1 accumulator — that needs the COMBINED 2×2 tx
  (two for-sale inputs in ONE tx), which is the remaining headline open item.

### 7. SALE v3 — SECONDARY-MARKET RESALE (buyer0 → buyer1) — LIVE
A peer-to-peer resale with Kyle (genesis owner) NOT a party. Proves the token CIRCULATES: a holder
who BOUGHT phoques can list and sell them with the identical tools, because the protocol treats every
holder as an equal peer (no privileged seller / no coordinator). This is the on-chain answer to "how do
buyers sell to other buyers."
- **(7a) buyer0 RESALE LISTING** tx: `d24437b8bddd37942076fbb608f9f05d21b4d094d17e2cb6fd1a046322a5c088`
  buyer0 signed (with `bitphoque_buyer.key`) to list their own 21034 at buyer0's OWN price 750000
  (a reseller sets their own price). Spent buyer0's not-for-sale UTXO `3265e063...:0`. For-sale UTXO
  `d24437b8...:0` at P2SH `kaspatest:prqhnhtk3vraw2fffvn3ezvmhaxqtv8nfxrfwm9v0wn9nu58kckxgyqwxq5xu`.
- **(7b) buyer1 CLAIMS buyer0's listing** tx: `1635fd2858223314ef4f0b350c63b2627fb18a0d4836aed69a2c4125b391b720`
  buyer1 paid 100M to **buyer0** (not Kyle), received 21034. The claim harness's party-agnostic seller
  arg = buyer0's pubkey `261cbf44...`. No Kyle signature, no Kyle involvement anywhere in this hop.
- CHAIN OF CUSTODY (21034 parcel): Kyle `50a1...` → buyer0 (6c) → buyer0 lists (7a) → buyer1 (7b).
- END STATE: buyer1 holds the whole supply (21034 from buyer0 + 21035 from Kyle = 42069). buyer0 net:
  paid 100M to acquire, received 100M reselling (TN10 practice values).
### 8. TRANSFER-SPLIT → realistic 3-holder distribution — LIVE
Generalized the self-split into a TWO-OWNER transfer-split (each output gets its own owner pubkey),
then seeded a realistic uneven distribution across three holders for future wallet/marketplace work.
NEW keys this session (generated + saved gitignored; buyer2/3 faucet-funded 1000 TKAS plain each):
- buyer2 = `5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161`
  (key `bitphoque_buyer2.key`; addr `kaspatest:qpwn8z49f085m92eg8lchnwsdxe082saadqzxnw88njlfavp4hckzcys2w2lh`)
- buyer3 = `267496ce01f2a35653491c61b56b23e6b7dbe777a02b146286ac3635ff36ab61`
  (key `bitphoque_buyer3.key`; addr `kaspatest:qqn8f9kwq8e2x4jnfywxrdtty0nt0kl8w7szk9rzs6krvd0lx64kzkhgpw6d5`)
- **(8a) split A** `81caee43...` : buyer1's 21034 → 1000 (buyer2, `81caee43...:0`) + 20034 (buyer1, `...:1`)
- **(8b) split B** `05160528...` : buyer1's 21035 → 10500 (buyer3, `05160528...:0`) + 10535 (buyer1, `...:1`)
- **(8c) split** `4f084e26...` : buyer1's 20034 → 1 (listing parcel, `...:0`) + 20033 (buyer1, `...:1`)
- **(8d) split** `0434801c...` : buyer1's 10535 → 5267 (listing parcel, `...:0`) + 5268 (buyer1, `...:1`)
- harness `rust-examples/bitphoque_sale_split_v3.rs` is the self-split; the two-owner generalization is
  `rust-examples/bitphoque_split_transfer_v3.rs` (CLI below). KEY FINDING: a split to a DIFFERENT owner
  is just a normal transfer — the contract constrains AMOUNTS (conservation), not output ownership, so
  the input owner can hand parcels to anyone. Verified at VM + on-chain.

### 9. COMBINED 2×2 CLAIM — THE ON-CHAIN F1-FIX PROOF — LIVE
The headline. buyer1 listed TWO of its parcels (same seller): `4943a956...:0` (1 phoque) and
`5aab21f6...:0` (5267 phoque), both price 500000. Then ONE tx claimed BOTH:
- **2×2 claim tx (ACCEPTED):** `40cbf8659db6e0d17465c0916da4581557483e8a5e3d204e91e12d085b79d80d`
- input 0 = for-sale `4943a956...:0` (1 phoque, seller buyer1), UNSIGNED, **witness → output 2**
- input 1 = for-sale `5aab21f6...:0` (5267, seller buyer1), UNSIGNED, **witness → output 3** (3 > 2)
- input 2 = buyer2 plain funding (buyer2-signed); input 3 = buyer3 plain funding (buyer3-signed)
- output 0 = 1 phoque → buyer2; output 1 = 5267 → buyer3
- output 2 = 100M → seller buyer1 (buyer2 pays); output 3 = 100M → seller buyer1 (buyer3 pays)
- outputs 4,5 = change to buyer2 / buyer3
- **What it proves:** two for-sale inputs from the SAME seller, in ONE tx, forced onto STRICTLY-
  INCREASING DISTINCT payment outputs. The v2 F1 hole (witnesses=[k,k] → two parcels, one payment)
  is structurally impossible here and confirmed on a real node. Same-seller is the strongest F1 test:
  cross-seller was always caught by the wrong-payee check (F2); same-seller was the actual hole.
- **Also proves atomic multi-party settlement:** two distinct buyers each fund their own payment in
  one atomic tx — the whole tx commits or none of it does; neither buyer can be rugged.
- harness `rust-examples/bitphoque_sale_claim_2x2_v3.rs` (hardcoded consts for this one-shot proof,
  not the general CLI). fee 1,500,000 cleared on first try (no floor correction needed).

FINAL DISTRIBUTION after the 2×2 (whole supply across 3 holders, all by genuine on-chain sales/splits):
- buyer1: 20033 + 5268 = 25,301   |   buyer2: 1000 + 1 = 1,001   |   buyer3: 10500 + 5267 = 15,767
- total 42,069 ✓

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
7. Claim fee floor ~654,600 (use 700,000) for the bigger 2-input, script-revealing tx (v2).

NOTE: genesis spends a PLAIN funding input as a rothschild bin and used SigopCount(1) successfully — the
ComputeBudget-on-all-inputs rule (#4) bit the CLAIM (a v1 covenant tx built via silverscript-lang), not genesis.

From the v3 SALE (list, split, and the 2-input claim — all on a real node):
8. **v3 listing/split fee floor ~659k (use 700k).** The 2793-byte v3 script raises tx mass vs v2's
   1969B floors. A single-input, single-output listing or a 1→2 split clears at 700k.
9. **v3 2-INPUT CLAIM fee floor = 857,000 (use 900k).** The claim is the HEAVIEST tx in the lifecycle:
   it reveals the 2793B script AND carries a second (buyer funding) input AND three outputs. The node
   rejected 700k with the exact required figure: `required amount of 857000 for compute mass 8570`.
   Bumping the harness FEE const to 900k cleared it. (List harness stays at 700k — it's lighter.)
10. **PAYMENT must dodge KIP-9 dust** (carried from v2 #6): pay 100M (>> the 500k/750k price) so the
    seller payment output isn't dust. Contract only requires payment >= price, so overpaying is fine.
### MULTI-COVENANT-INPUT ABI (leader / delegate) — the combined-claim finding
For a tx with N>1 covenant inputs, the covenant ABI is leader/delegate (confirmed against the proven
merge pattern in `silverscript-lang/tests/kcc20_tests.rs` lines 289–304):
- **Input 0 = LEADER:** `build_sig_script_for_covenant_decl("transfer", vec![newStates, sigs, witnesses],
  { is_leader: true })`. It carries the COMPLETE arrays for ALL covenant inputs/outputs —
  newStates (one per covenant OUTPUT), sigs (one per covenant INPUT), witnesses (a FLAT byte[],
  one byte per covenant input).
- **Every other covenant input = DELEGATE:** `build_sig_script_for_covenant_decl("transfer", vec![],
  { is_leader: false })` — EMPTY args. The delegate still pushes its own redeem script.
- The contract reads `witnesses[i]`, `prevStates[i]`, `newStates[i]` from the LEADER's arrays. If you
  (wrongly) make every input a leader with its own single-element arrays, the contract slices past the
  end of the short per-input array and fails `OutOfBoundsSubstring(1, 2, 1)` (it tried to read
  witnesses[1] from a 1-byte blob). This was the exact bug hit + fixed building the 2×2 claim.
- witnesses encoding: a single byte[] blob, e.g. `Expr::bytes(vec![2u8, 3u8])` — NOT a 2-element array
  of separate byte pushes. The contract does `int(witnesses[i])` to read each index.

### Fee floor (add to the list)
- v3 COMBINED 2×2 claim (4 inputs: 2 covenant + 2 plain; 6 outputs; TWO script reveals) cleared at
  fee 1,500,000 on the first try. Floor is somewhere below 1.5M; this is the heaviest tx in the
  lifecycle. (Single 2-input claim floor was 857k/use 900k; the second covenant input + script reveal
  is the jump.)

### NODE / INDEXING QUIRKS
- **PsychoNode address-index LAGS the tx-index by a few seconds after confirmation.** A claim run
  immediately after a listing confirmed returned "no for-sale covenant UTXO found" against the CORRECT
  P2SH (verified on the explorer that the listing output WAS there). Re-running the same command a
  moment later found it. The compiled P2SH was right the whole time — the node's by-address UTXO
  lookup simply hadn't caught up. IMPLICATION for the future wallet/inspect tool: poll/retry on
  by-address lookups, or query by OUTPOINT (which uses the tx-index) rather than by address.

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
- **Sale model (v2/v3):** per-UTXO `forSale` (bool) + `price` (int) state. Listing is owner-signed; claiming is
  payment-authorized. checkSaleState allows for-sale OUTPUTS only when NO input is for-sale (a listing);
  forbids them when ANY input is for-sale (a claim — anti-double-claim). v3 adds the strict-increasing
  payment-output rule (F1 fix). DECIDED: per-UTXO (fungible), NOT per-phoque ids (which would make it an NFT
  collection). Full design + open items: `bitphoque_swap_covenant_design.md`.

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

### Sale token v3 — `covenants/bitphoque_sale_v3.sil` (DEPLOYED as the live sale token)
Same 8-arg ctor + state shape as v2; adds the F1 fix. Script 2793 bytes. covenant_id `3baed5a3...`.
- F1 fix: for-sale inputs must use STRICTLY-INCREASING payment-output indices (lastSaleWitness accumulator
  seeded at -1; each for-sale witness must exceed the previous). No two for-sale inputs share an output.
- Caps 3/3 baked at genesis (must match across split/list/claim harnesses — amount/caps bake into the P2SH).

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
- **Sale + listing gate** (`rust-examples/bitphoque_sale_reject_tests.rs`, against v3): all green — 10× 1x1
  (C-CONTROL..C6, L-CONTROL..L2) + 2x2 controls (claim, split-on-list) + 5 fanout cheats rejected (F1 double-claim,
  F2 cross-wire, F3 inflate, F4 claim-leaves-listing, F5 mixed-auth) + 3x3 fanout (G-CTRL, G1..G4).

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

### Party-agnostic v3 harnesses (the seed of the planned CLI)
- `bitphoque_sale_list_v3.rs` — CLI: `<amount> <price> <owner_keyfile> [--go]`. Owner KEYFILE is an arg
  (listing is owner-signed — needs the key). Each amount → a different P2SH, so amount is required.
- `bitphoque_sale_claim_v3.rs` — CLI: `<seller_pubkey_hex> <amount> <price> <buyer_keyfile> [--go]`. Seller is
  identified by PUBKEY only (claiming is payment-authorized — no seller key needed). This is what lets any
  holder resell with the same tool. FEE const = 900k (claim floor 857k).

---

## File tree (`C:\bitphoque`)
```
C:\bitphoque\
  .gitignore                           <- ignores *.key (all key files)
  bitphoque_owner.key                  <- original token owner / v2 FUNDING signer (GITIGNORED)
  bitphoque_v2_owner.key               <- v2/v3 sale token owner = SELLER 50a1... (GITIGNORED)
  bitphoque_buyer.key                  <- buyer0 261cbf44... (GITIGNORED)
  bitphoque_buyer1.key                 <- buyer1 8c02c20a... (GITIGNORED)
  bitphoque_buyer2.key                 <- buyer2 5d338aa5... (GITIGNORED)
  bitphoque_buyer3.key                 <- buyer3 267496ce... (GITIGNORED)
  rust-examples\
    bitphoque_split_transfer_v3.rs     <- TWO-OWNER transfer-split (CLI: in_amount, signer_keyfile,
                                          out0_amount, out0_pubkey, out1_amount, out1_pubkey)
    bitphoque_sale_claim_2x2_v3.rs     <- COMBINED 2×2 claim — the on-chain F1 proof (hardcoded consts)
  covenants\
    bitphoque.sil                      <- original token contract (KCC20 verbatim, DEPLOYED)
    bitphoque_ctor.json / _ctor_real.json / _compiled_*.json  <- original ctors + compiled
    bitphoque_sale_v1.sil              <- sale contract v1 (SUPERSEDED — listing was impossible)
    bitphoque_sale_v2.sil              <- sale contract v2 (DEPLOYED as the v2 sale token)
    bitphoque_sale_v3.sil              <- sale contract v3 (DEPLOYED as the live sale token; F1 fix)
    bitphoque_sale_ctor.json           <- v2 sale ctor (test/8-arg shape)
    bitphoque_sale_genesis_ctor.json   <- v2 genesis ctor (real owner pubkey baked in)
    bitphoque_sale_compiled.json       <- v2 compiled (sale_genesis harness reads script from here)
    bitphoque_sale_compiled_v3.json    <- v3 compiled (genesis_v3 harness reads script from here)
    (abandoned 520-investigation experiments: bitphoque_slim.sil, *_1x1.json — kept as evidence)
  rust-examples\
    bitphoque_keygen.rs                <- keypair generator
    bitphoque_genesis_establish.rs     <- original genesis harness
    bitphoque_transfer.rs              <- transfer/handoff harness (LIVE transfer)
    bitphoque_reject_tests.rs          <- original transfer gate proof
    bitphoque_inspect.rs               <- read-only on-chain confirm tool
    bitphoque_sale_reject_tests.rs     <- sale+listing gate proof (claim + listing, all cheats; v3)
    bitphoque_sale_genesis.rs          <- v2 genesis harness (LIVE v2 genesis)
    bitphoque_sale_genesis_v3.rs       <- v3 genesis harness (LIVE v3 genesis)
    bitphoque_sale_split_v3.rs         <- v3 split harness (1->2; LIVE split)
    bitphoque_sale_list.rs             <- v2 LISTING harness (LIVE v2 listing)
    bitphoque_sale_claim.rs            <- v2 CLAIM harness (LIVE v2 claim)
    bitphoque_sale_list_v3.rs          <- v3 LISTING harness, PARTY-AGNOSTIC (LIVE v3 listings + resale)
    bitphoque_sale_claim_v3.rs         <- v3 CLAIM harness, PARTY-AGNOSTIC (LIVE v3 claims + resale)
  docs\
    bitphoque_status.md                <- this file
    bitphoque_swap_covenant_design.md  <- sale/swap design + auction future-build + open items
    bitphoque_mainnet_checklist.md     <- the pre-mainnet GATE
    operating_baseline.md              <- shared portable baseline (copy in oracle tree)
      
```
Own git repo (branch `master`). NOT yet pushed to GitHub — later deliberate "make it public" pass
(clean repo name + README + secret-sweep first).

---

## WALLET / CLI — DECISION & DIRECTION (2026-06-19)

The end-goal framing Kyle set this session (keep these three goals in view for all BitPhoque work):
1. **Fund the ongoing Oracle Protocol / related work by selling phoques over time.**
2. **Master covenants + post-Toccata Kaspa.**
3. **Help drive Kaspa adoption.**

Wallet/CLI sequencing DECIDED (engine before chassis):
- **Engine first** — finish proving the sale lifecycle on-chain (done for separate claims + resale;
  combined 2×2 still open). Don't divert to UI before the claim path is proven repeatable.
- **Then collapse harnesses → one party-agnostic CLI** (`bitphoque list/claim/balance …`). The two
  party-agnostic harnesses are the seed; the refactor is mostly argument plumbing + a shared lib.
- **Then a thin BitPhoque wallet APP** over that lib — start with the ONE screen that matters: a
  marketplace view ("here are listings, click to buy"). A CLI is fine for Kyle (minting/listing his
  supply); a real buyer reselling needs a button, not `cargo run`. The wallet is what makes
  peer-to-peer real for non-technical holders.
- **General covenant-token wallet** = its own (later) project; arguably THE product if covenant tokens
  take off on Kaspa. Not now.
- **The indexing problem is the wallet's hard part** (and overlaps the inspect-generalization item):
  each (owner, amount, forSale, price, caps) tuple is a DIFFERENT P2SH, so a wallet can't watch one
  address per key — it must enumerate candidate P2SHes or index covenant UTXOs by covenant_id. The
  harnesses dodge this because they always KNOW the amount and compile the exact P2SH. Design this
  before building the app (and mind the address-index lag quirk above).

---

## TN10 ground facts
- Node: PsychoNode at 192.168.0.206 — wRPC borsh 17210, gRPC 16210. Release tn10-toc3.
- Funding addr (reused for both genesis runs): `kaspatest:qrwzyuc8lkvdsxgww53n7rsjetsqhh6p9hmy8vrvqpdun5r7ysevufmglhgl4`
  (key in bitphoque_owner.key; re-faucet-funded 100k TKAS for the v2 genesis).
- rusty-kaspa clone: C:\kaspa-tn12\rusty-kaspa (tag tn10-toc3); silverscript: C:\kaspa-tn12\silverscript.
- Faucet: min 1000 TKAS per request, 24h cap 100k. Fund wallets 1000 at a time so the cap is never a constraint.
- Fee floors learned: genesis ~167k (use 200k); transfer ~378,600 (use 400k); v2 claim ~654,600 (use 700k);
  v3 list/split ~659k (use 700k); v3 2-input claim 857,000 (use 900k).

---

## RESUME POINT — combined 2×2 claim (on-chain F1 proof) is the headline; then generalize + mainnet

Fixed-price sale PROVEN end-to-end on-chain: 1x1 on v2, and on v3 at 2×2 as TWO SEPARATE CLAIMS plus a
SECONDARY-MARKET RESALE. 2x2 AND 3x3 fanout proven AT THE VM on v3 — the work that found and fixed the F1 bug.

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

Remaining, in order:
- [x] **Re-prove list+claim ON-CHAIN at 2×2 (two SEPARATE claims)** — DONE. Two listings
      (cebf75b5, 55542a99), two independent claims by two distinct buyers (3265e063 buyer0/21034,
      9ffe84b8 buyer1/21035). The "sell over time to successive buyers" use case is live.
- [x] **Secondary-market RESALE proven** — DONE. buyer0 listed bought phoques (d24437b8) and buyer1
      claimed them (1635fd28); Kyle not a party. Token circulation confirmed on-chain.
- [x] **List + claim harnesses made PARTY-AGNOSTIC** — DONE. Both take party identity at the CLI
      (list: owner KEYFILE since listing is owner-signed; claim: seller PUBKEY since claiming is
      payment-authorized and needs no seller key). This is the seed of the planned CLI/wallet.
- [x] **3x3 fanout PROVEN at VM** (G-CTRL claim/split, G1 triple-share, G2 mid-collision, G3 out-of-order,
      G4 non-contiguous all behave). NxM inferred (accumulator is N-agnostic) but only 2 and 3 tested.
      NOTE: strict-increasing rejects DISTINCT-but-UNORDERED witnesses (G3) — safe but stricter than
      minimal; per-output-accounting mainnet version would relax it.
- [x] **COMBINED 2×2 claim tx (the on-chain F1-fix proof)** — DONE. tx `40cbf865...`: two same-seller
      for-sale inputs forced onto strictly-increasing payment outputs (witnesses=[2,3]), two buyers
      each funding their own input. F1 double-claim is now structurally impossible AND proven on-chain.
- [x] **Two-owner transfer-split** — DONE (`bitphoque_split_transfer_v3.rs`). Seeded a realistic
      3-holder distribution (buyer1/2/3). Split-to-different-owner verified as a normal transfer.
- [ ] **Split/merge generalization for the sale** (list part of a balance; MERGE N→1 still unproven
      on-chain — the leader/delegate ABI for multi-input is now proven via the 2×2 claim, so merge is
      a near neighbor).
- [ ] (unchanged) generalized inspect tool; wallet/CLI build (engine now fully proven — collapse
      harnesses → party-agnostic CLI → thin marketplace app); original-token owner quirk; AUCTION
      future build; mainnet checklist; make public.
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
