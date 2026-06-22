# BitPhoque — Status

_**Read `operating_baseline.md` alongside this doc** — the same working discipline applies: one
command at a time, single-line terminal commands, plain-English-before-code, source-of-truth files
committed to git, dry-run before broadcast, verify-by-reading-source for anything that enforces,
local VM verify before TN10. BitPhoque lives in its OWN tree (`C:\bitphoque`) and its OWN git repo —
separate from oracle-protocol. The end-of-session git checkpoint ritual covers TWO trees._

_Last updated: **WALLET ARCHITECTURE DESIGN SESSION (2026-06-22, no code/commit yet — design only).** A working
session spent on the USER-FACING wallet (the "thin marketplace app" successor), entirely while the TN10 node was
mid-IBD so nothing on-chain could be tested. Decisions settled (full detail in the new "USER WALLET — ARCHITECTURE
(2026-06-22)" section below): (a) FORM = a web app at BitPhoque.org — one codebase, all four platforms via browser
now, Tauri (desktop) / Capacitor (mobile) wraps later; do NOT start with four native apps. (b) CUSTODY =
non-custodial, standard BIP-39 12-word seed, key generated + held in-browser, never on a server; Guardian/Circle
social recovery PARKED as a planned opt-in upgrade (preferentially covenant-layer key-rotation, NOT seed-Shamir).
(c) The RECEIVE PROBLEM (a parcel's P2SH is amount-derived, so a fresh wallet can't discover holdings blind) is
solved by a HYBRID: an off-chain signed PARCEL DESCRIPTOR (the trustless protocol truth) + an OPTIONAL indexer
(convenience cache, anyone can run one), chain always the authority. Descriptor format specced (base58check
transport). (d) The wallet MUST handle native KAS to pay fees — "gas-tank, not vault" posture (size for fees +
headroom, steer savings to a main wallet) to keep the browser-key risk surface small. (e) ONE token mechanism:
BitPhoque's own KCC-20-style COVENANT token (confirm the "KCC-20" name against `kcc20.sil` in-tree) — NOT KRC-20
(different, indexer-based mechanism; PARKED as a separate later question, verify tooling before committing). (f)
Build the narrow BitPhoque wallet but keep the KAS layer SEPARABLE ("send a funded Kaspa tx" as a clean primitive
with parcel logic on top) so a general Kaspa wallet stays reachable without being built now. NB: this session
produced NO files and NO commits — it is design captured into this doc only; nothing is git-protected beyond this
doc edit. Engine status UNCHANGED from below: claim's live dry-run still gated on the node finishing IBD (node had
just finished receiving the UTXO set and begun "Importing the UTXO set of the pruning point" at session end — past
the chunk-download phase, not yet synced)._

_Prior milestone — **CLI FEATURE-COMPLETE (claim committed) + TN10 NODE TOCCATA UPGRADE.** (1) CLI: **`claim` committed**
(`78c14d9`) — CLI now has import / balance / list / split / merge / partial-list / **claim**; ALL subcommands built,
compiling, committed. The harness-collapse phase is DONE in code. Remaining gate on claim: a LIVE dry-run VM-verify,
blocked only on the TN10 node finishing IBD. (2) **TN10 node root-caused + rebuilt.** The TestnetNode kept wedging at
the utxoindex/consensus-init step — root cause was a **mainnet-line kaspad 2.0.0 binary + a pre-Toccata datadir**
(consensus store last written June 9; Toccata hit TN10 ~June 20). Fix: upgraded to **kaspad 2.0.1** (the post-Toccata
floor per the Toccata guide), wiped to a fresh datadir (`datadir_pretoccata_old` set aside), launched clean —
now IBD'ing healthily on the correct rules with gRPC reachable. (3) **gRPC fix:** the TN10 launch .bat was missing
an external gRPC listener (only had `--rpclisten-borsh`), so the CLI (which uses GrpcClient) couldn't connect.
Added `--rpclisten=0.0.0.0:16210`; confirmed `GRPC Server starting on: 0.0.0.0:16210` on launch. **CLI NODE const
must be `192.168.0.104:16210`** (gRPC), NOT 16211 (that's P2P — the old "gRPC 16211" note was WRONG). After IBD:
live claim dry-run → shared lib → thin marketplace app → mainnet checklist._

_Prior milestone — **PARTIAL-BALANCE LISTING PROVEN ON-CHAIN.** One owner-signed tx now lists an arbitrary
PART of a holding (`4b0c8187...`): 5000 of buyer1's 25301 merge parcel put up for sale @ 500000 sompi,
20301 kept as not-for-sale change, same owner — collapsing the old two-step split-then-list into one tx.
Harness `bitphoque_partial_list_v3.rs` (minimal diff off the proven self-split: out0 for-sale at a CLI
price, both outputs same owner, proportional value split). LOCAL VM VERIFY confirmed the listing branch
accepts a mixed-state 2-output fanout (one for-sale + one not-for-sale) on an owner-signed tx, exactly as
checkSaleState reads. Remaining: wallet/CLI collapse, then mainnet checklist._

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
- **Next:** the wallet/CLI is FEATURE-COMPLETE — `bitphoque_cli.rs` has import / balance / partial-list / list /
  split / merge / **claim**, all built + committed (`78c14d9` claim), on the Option-C track-forward model. Only
  remaining gate on claim is a LIVE dry-run VM-verify (waiting on TN10 IBD). Next up the
  list: live claim dry-run, extract a shared lib, then the thin marketplace app,
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

### 10. MERGE 2→1 (consolidate parcels) — LIVE
The inverse of the split: TWO not-for-sale, SAME-owner covenant UTXOs collapsed into ONE. First
on-chain covenant MERGE for BitPhoque. buyer1 held two parcels (20033 + 5268) at two different
P2SHes; merged into one 25301 parcel. Owner-SIGNED on BOTH inputs (both not-for-sale → checkAuth's
checkSig branch). Conservation enforced (totalIn == totalOut). 2-covenant-input tx → leader/delegate
ABI (same machinery as the 2×2 claim, but signed not payment-authorized).
- **merge tx (ACCEPTED):** `519063be8d37e7768ba93e54c517ced3fb32eba89b7194b3c15bf0e72fe0f15c`
- input 0 = `4f084e26...:1` (20033, buyer1), owner-SIGNED, LEADER
- input 1 = `0434801c...:1` (5268, buyer1), owner-SIGNED, DELEGATE
- output 0 = `519063be...:0` = 25301 phoques, value 824,996,900,000 sompi (= both inputs − 1.3M fee),
  buyer1, not-for-sale
- BOTH old parcels confirmed SPENT on-chain (inspect_v3 returned empty for 20033 and 5268 after merge).
- harness `rust-examples/bitphoque_sale_merge_v3.rs` (CLI: `<amount0> <amount1> <owner_keyfile> [--go]`).
- **THE TWO-SIGNED-COVENANT-INPUT FINDING** (the bug that cost one VerifyError + the fix): both owner
  sigs are computed against the LEADER input's sighash (INDEX 0), NOT against each input's own index.
  The covenant verifies ALL input sigs from the leader's position. Signing sig1 against input 1 gave a
  different sighash → VerifyError. Fixed by signing both against input 0, per the proven kcc20_tests.rs
  merge pattern (which signs both at index 0 and uses witness_array [0,1]). This is the FIRST tx that
  signs two covenant inputs — the 2×2 claim's covenant inputs were UNSIGNED (payment-authorized), so it
  never surfaced there. Carry this forward to the CLI/wallet: any owner-signed multi-covenant-input tx
  (merge, multi-parcel transfer) signs every covenant input at the leader index.

### Generalized inspect tool — `rust-examples/bitphoque_inspect_v3.rs` (BUILT this session)
Read-only. Compiles ANY v3 parcel's P2SH in-process from CLI params (owner pubkey + amount + forSale +
price; same ctor shape as the list harness) and queries the node. Takes a PUBKEY (never a keyfile) —
stream-safe, no key material. CLI: `<owner_pubkey_hex> <amount> <forsale:0|1> <price> [--outpoint
txid:index] [--retries N]`. Supersedes the hardcoded-address rothschild `bitphoque_inspect.rs` for v3
parcels (that one still works for the original v1 token). Used 3× this session to confirm the merge
(parcels live pre-merge, 25301 result present, both inputs spent). Silverscript-clone bin (needs the
compiler API to build the P2SH).

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
- **SIGNED multi-covenant-input (the merge finding):** when the covenant inputs are owner-SIGNED (not
  payment-authorized — i.e. not-for-sale inputs taking checkAuth's checkSig branch), EVERY input's
  signature is computed against the LEADER input's sighash (INDEX 0), NOT against each input's own
  index. The covenant verifies all sigs from the leader's position. Signing input 1's sig against
  input 1's sighash → VerifyError. Confirmed against the proven kcc20_tests.rs merge pattern, which
  signs BOTH inputs at index 0 and passes witness_array [0,1]. The 2×2 claim did not expose this — its
  covenant inputs were UNSIGNED (dummy sigs; auth came from payment outputs). Merge is the first
  two-signed-covenant-input tx. RULE for the CLI/wallet: any owner-signed multi-covenant-input tx signs
  every covenant input at the leader index (0).

### Fee floor (add to the list)
- v3 COMBINED 2×2 claim (4 inputs: 2 covenant + 2 plain; 6 outputs; TWO script reveals) cleared at
  fee 1,500,000 on the first try. Floor is somewhere below 1.5M; this is the heaviest tx in the
  lifecycle. (Single 2-input claim floor was 857k/use 900k; the second covenant input + script reveal
  is the jump.)
- v3 MERGE 2→1 (2 covenant inputs, TWO 2793B script reveals, 1 output): node-confirmed floor
  **1,244,500** (compute mass 12445); use **1.3M**. HEAVIER than the single 2-input claim (857k)
  because the 2nd input is a FULL script reveal, not a cheap plain input — two 2793B reveals dominate
  the mass. Lighter than the 2×2 claim (1.5M) because no plain funding inputs / extra outputs.

### NODE / INDEXING QUIRKS
- **Address-index LAGS the tx-index by a few seconds after confirmation** (observed on the OLD PsychoNode
  hardware, now retired; the new more-capable TestnetNode @ .104 may lag less, but treat the retry as still
  required until re-confirmed). A claim run
  immediately after a listing confirmed returned "no for-sale covenant UTXO found" against the CORRECT
  P2SH (verified on the explorer that the listing output WAS there). Re-running the same command a
  moment later found it. The compiled P2SH was right the whole time — the node's by-address UTXO
  lookup simply hadn't caught up.
- **CORRECTION (verified against rusty-kaspa RpcApi source this session):** there is NO by-outpoint
  UTXO query on the RpcApi trait — `get_utxos_by_addresses` is the ONLY UTXO lookup. An earlier note
  here suggested "query by outpoint (tx-index, no lag)"; that RPC does not exist for this client. The
  ONLY available mitigation for the lag is RETRY on the by-address lookup (optionally filtering the
  returned set to a specific outpoint CLIENT-SIDE). `bitphoque_inspect_v3.rs` implements this: a retry
  loop (`--retries N`, 3s between attempts) plus an optional `--outpoint` client-side filter. Confirmed
  working this session — after the merge, inspect_v3 took 2 retries before the address index showed the
  old parcels as spent. The future wallet/inspect generalization must poll/retry, not assume a
  by-outpoint shortcut.
- **LOADED-NODE LAG SCARE (2026-06-20) + the diagnostic that settled it.** During the CLI thin-slice
  proof, a freshly-created partial-list's outputs (`8dff6df4...:0/:1`) read as NOT FOUND from both
  `balance` and `inspect_v3`, even after retries — looked like the tx had misbuilt or the outputs landed
  at the wrong P2SH. It had NOT. `bitphoque_node_diag.rs` (read-only health + old-vs-new probe, same
  connection) showed: node synced=true, utxo_indexed=true, BUT mempool ~62,757 (very heavy load) — and
  on a settled query ALL parcels (old 5000, new 3000, new 17301) were FOUND, the spent input (20301)
  correctly absent. Conclusion: the misses were pure address-index lag under mempool load, not a bug. The
  P2SH compute was correct the whole time (diag computed the SAME addresses inspect/CLI did — proven by
  the 20301 address matching byte-for-byte). LESSON: when a fresh parcel "vanishes," probe node health +
  a known-OLD parcel before suspecting the tx. `bitphoque_node_diag.rs` is the keep-around tool for this.

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
- **Subdivision — BitPups (DECIDED IN PRINCIPLE; bake in at MAINNET GENESIS).** A phoque subdivides into a named
  base unit, the **BitPup** (seal-pup; mirrors BTC→satoshi, KAS→sompi). Convention: **1 phoque = 100,000,000
  BitPups (8 decimal places)**, matching BTC/KAS. APPROACH = pure denomination, NO contract change ("option 1"):
  the contract already tracks `amount` as an integer and enforces conservation on whole integer units, so we simply
  declare the integer unit to BE the BitPup. "1 phoque" becomes a display-layer label for 10^8 BitPups — exactly
  how Bitcoin only ever moves satoshis and "BTC" is the human label. The contract is UNIT-AGNOSTIC: it can already
  send any whole BitPup amount (e.g. amount=50,000,000 = half a phoque), conservation holds, no fractional logic
  needed. CONSEQUENCE: at mainnet genesis the supply integer becomes 42,069 × 10^8 = **4,206,900,000,000,000
  BitPups** (well within i64's ~9.2e18 ceiling — checked, no overflow). CONSTRAINT: this is a GENESIS decision,
  permanent and unchangeable after mint, and it CANNOT be retrofitted to the live TN10 token (minted as 42,069 whole
  units with no sub-unit headroom — would need re-genesis). Decide once, correctly, at mainnet day one. Wallet/CLI
  display divides by 10^8 to show phoques; no covenant work required.
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
  .gitignore                           <- ignores *.key (all key files) + wallet.json (local CLI state)
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
    bitphoque_inspect_v3.rs            <- GENERALIZED inspect (compiles any v3 P2SH from params; retry
                                          loop for index lag; read-only, pubkey-only). committed 46e4568
    bitphoque_sale_merge_v3.rs         <- MERGE 2→1 harness (LIVE merge 519063be...). committed 46e4568
    bitphoque_partial_list_v3.rs       <- PARTIAL-LIST harness (LIVE 4b0c8187...): lists PART of a
                                          balance in one owner-signed tx (out0 for-sale + out1 change)
    bitphoque_cli.rs                   <- WALLET CLI thin slice: import / balance / partial-list, on the
                                          Option-C track-forward model (reads/writes wallet.json)
    bitphoque_node_diag.rs             <- read-only node health + index-lag probe (old-vs-new parcel)
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

### WALLET CLI — THIN SLICE BUILT (2026-06-20), Option-C model
The indexing problem above is SOLVED for the near-term wallet via **Option C: track-forward**. The
wallet does NOT scan the DAG — it maintains its OWN parcel set in a local `wallet.json` (project root,
GITIGNORED, NEVER holds private keys — owner is a pubkey; signing reads a `*.key` by PATH; safe on
stream). Each record carries exactly the ctor args that determine the P2SH (owner, amount, forSale,
price, caps) + outpoint + value + status, so the wallet can always RECOMPUTE the P2SH and confirm the
parcel live — it never has to GUESS an amount. A parcel enters the wallet three ways: you created it
(CLI records the outputs as it broadcasts), you received it (the tx names the amount), or you `import`
it (the bootstrap + rescan-by-hint recovery path). Honest limit: a parcel arriving via a tx the wallet
DIDN'T build won't be known until `import`. Full Option-C upgrade path = a real DAG indexer (deferred,
"its own project").

`bitphoque_cli.rs` — single-file thin slice, three subcommands, same safety spine as every harness
(dry-run default, `--go`, LOCAL VM VERIFY gates broadcast; tx-build code lifted verbatim from the
proven partial-list harness):
- `import  <owner_pk_hex> <amount> <forSale 0|1> <price>` — confirm a parcel on-chain (retry through
  index lag) and record it. Overwrites an existing record (so it doubles as the repair path).
- `balance <owner_pk_hex>` — read wallet.json, recompute each live parcel's P2SH, confirm against the
  node, print holdings (total + per-parcel, for-sale flagged). PURE READ — NEVER writes status on a
  lookup miss (a miss is ambiguous: index lag vs spend), just flags "not confirmed". Filters status=spent.
- `partial-list <owner_keyfile> <in_amount> <list_amount> <price> [--go]` — the proven partial-list, and
  on ACCEPTED it records FORWARD: input marked spent, out0 (for-sale) + out1 (change) recorded live.

PROVEN on-chain via the CLI: partial-list tx `8dff6df4fa58e6efa72998d4914a7b387c8b0bacddccb1a1e61ebfc4c96f7a48`
— 3000 of buyer1's 20301 listed @ 500000 (out0) + 17301 change (out1). balance then showed all three of
buyer1's live parcels (5000 + 3000 for-sale + 17301 held = 25301).

BUGS FOUND + FIXED this slice (both real, both in `bitphoque_cli.rs` now):
1. **wallet.json parser separator.** `save_wallet` joins records with `},\n {` (comma + whitespace), but
   the reader split on bare `},{` and missed the comma — saw the file as ONE record, dropped the rest.
   Fix: the reader skips comma AND whitespace at the `}`→`{` boundary. (Lesson: debug from the ACTUAL
   on-disk bytes — `type wallet.json` — not an assumed format. First fix attempt missed the comma.)
2. **balance auto-marked-spent on a lookup miss.** Under heavy mempool load the address index lagged, a
   fresh parcel read as NOT FOUND, and balance WRONGLY wrote status=spent — corrupting wallet.json. Fix:
   balance is now non-destructive; a miss is reported, never written. (See the LOADED-NODE LAG SCARE in
   NODE/INDEXING QUIRKS — `bitphoque_node_diag.rs` proved the parcels were fine, just lagging.)

DELIBERATE SHORTCUT (noted so it doesn't become the design): the wallet.json parser is hand-rolled (reads
only its own flat output), to avoid a serde dep in the thin slice. When the shared lib is extracted for the
full CLI, swap in serde_json.

CLI SUBCOMMANDS COMPLETE: **claim is PORTED + COMMITTED** (`78c14d9`) — joining list/split/merge/partial-list
(`9847e00` merge, `2211f09` split). CLI = import / balance / partial-list / list / split / merge / claim, all
compiling + committed. claim's port notes: arg order is `<buyer_keyfile> <seller_pubkey_hex> <amount> <price>`
(keyfile-first, matching list/split/merge convention); single covenant input so NO leader/delegate quirk (buyer's
plain input signs its own index 1); local CLAIM_FEE=900k / CLAIM_SCRIPT_UNITS=300k / PAYMENT=100M consts scoped in
the fn (like merge) so they don't disturb the shared 700k/240k calibration; forward-records the claimed parcel to
wallet.json. REMAINING on claim: a LIVE dry-run VM-verify on-chain (every other subcommand got one; claim's was
blocked on the node). Then extract the shared lib; then the thin marketplace app.

---

## USER WALLET — ARCHITECTURE (2026-06-22, DESIGN ONLY — no code, no commit)

This is the design for the END-USER wallet — the successor to the "thin marketplace app" in the sequencing
section above, aimed at NON-TECHNICAL holders (a buyer reselling phoques needs a button, not `cargo run`). The
whole session ran while the TN10 node was mid-IBD, so this is pure design — NOTHING here is built or tested
on-chain yet. The `bitphoque_cli.rs` thin slice (above) is the proven ENGINE this wallet wraps; this section is
the CHASSIS design that sits on top of it.

### Form factor — DECIDED: web app at BitPhoque.org
- A single web app, served from BitPhoque.org, IS the wallet (no install, no app-store gate, no `cargo build` for
  the user). Runs on Windows / Mac / Android / iOS today via the browser — ONE codebase.
- Native wraps come LATER off the same codebase: Tauri (or Electron) for desktop binaries, Capacitor for the
  Android/iOS store apps. Explicitly DO NOT start by building four native apps — that's the top of the difficulty
  ladder (iOS especially: paid dev account, app review, crypto-app scrutiny), not the start of it.
- HONEST ORDER-OF-MAGNITUDE (difficulty ladder, cheapest→dearest): CLI (done) → local web app talking to the node
  → hosted web app with client-side (WASM) tx-building → desktop wrap → mobile wrap → iOS store. Each rung is a
  real jump, not packaging.
- The site is a DELIVERY mechanism for the app, NOT a trust root. The running wallet verifies against the CHAIN;
  the site serving it must never become the thing that TELLS the wallet what it owns (that would rebuild the
  coordinator the whole project avoids).

### Custody — DECIDED: non-custodial, BIP-39 12-word, key in-browser
- Standard BIP-39 12-word seed. Universal, proven, mature Rust tooling, built-in checksum, WASM-friendly. Chosen
  deliberately over a novel scheme (don't reinvent crypto; smallest diff from proven). Key generated and held
  CLIENT-SIDE (browser), never on a server. Anthropic/BitPhoque.org never holds keys.
- v1 STANCE (chosen, not drifted-into): the user owns their backup exactly as in any standard wallet — back it up
  or be out of phoques. Acceptable for a TN10 novelty token; the seed-reveal screen makes the backup step
  deliberately un-skippable (write-down + confirm) because that's where non-technical users get hurt (silent,
  delayed failure).
- GUARDIAN / CIRCLE social recovery = a PLANNED OPT-IN UPGRADE, not v1. Explored this session: Shamir/SLIP-39
  splitting of the seed works but (i) dedicated SLIP-39 Rust crates are weak/WIP/deprecated, (ii) SLIP-39 is
  SatoshiLabs-controlled + not BIP-39 round-trip compatible, (iii) plain Shamir loses the checksum/typo-detection
  you'd then have to rebuild. CONCLUSION: the on-brand ("Everyone is an Oracle") recovery lives at the COVENANT
  layer — guardians authorize an on-chain key-rotation under a quorum, so the key is NEVER reassembled in plaintext
  (avoids Shamir's reconstruction-exposure window) — but that is UNPROVEN on this stack (depends what the
  silverscript covenant can verify; CSFS is in the Oracle work). Park as the ambitious-but-unverified upgrade;
  do NOT block v1 on it. HARD RULE confirmed: never derive a key from anything the user MEMORIZES (brain wallets
  are catastrophic — humans don't generate 256 bits).

### The receive problem + the descriptor (the keystone)
The defining constraint (already known, see "indexing problem is the wallet's hard part" above): a parcel's P2SH
is derived from its CONTENTS (owner pubkey, amount, forSale, price). To find a UTXO you must already know the
amount. So "someone sent me phoques" is NOT self-evident to a fresh wallet the way an incoming KAS payment is.
DECIDED solution = HYBRID (the only one that serves non-technical users AND the self-erecting lens):
- **Parcel descriptor (the trustless protocol truth).** An off-chain "receipt" the sender hands the receiver,
  carrying exactly what's needed to FIND and VERIFY the parcel. It is essentially the `inspect_v3` input args made
  portable. Fields (v1, draft layout — all load-bearing): format-version byte ‖ covenant_id (32B) ‖ owner_pubkey
  (x-only 32B) ‖ amount ‖ forSale (1B) ‖ price ‖ txid (32B) ‖ output_index.
- **Transport = base58check** (the Bitcoin-address alphabet — no confusable 0/O/I/l — WITH a checksum baked into
  the standard, so a hand-transcribed receipt fails LOUDLY instead of computing a wrong P2SH silently). This
  SUPERSEDES the earlier "add a separate checksum field" idea — base58check IS the checksum, one fewer thing to own.
  QR and a `bitphoque:` URI are trivial wrappers over the same base58 string, deferrable.
- **Optional indexer (convenience cache, NOT authority).** Watches the chain, decodes BitPhoque parcels, answers
  "parcels for pubkey X." The marketplace app's job. A wallet works FULLY on descriptors alone (trustless,
  coordinator-free); pointed at ANY indexer it works PLEASANTLY (balances just appear). Because the chain is always
  the final verifier, a malicious indexer can lie about what EXISTS but can never forge a parcel you don't control
  (Electrum-server trust model). This is what keeps the convenience layer from becoming a control point.
- A descriptor is a SNAPSHOT, not a permanent address: the instant a parcel is spent (list/claim/split/merge) its
  outpoint dies and its P2SH may move. Wallet rule: re-derive live state against the chain on open; treat the
  descriptor as a hint to VERIFY, never a cached account number.
- OPEN (small, closeable): pin the integer encoding for amount/price/output_index in the descriptor. LEANING
  8-byte little-endian to match the existing ctor-arg serialization (and Oracle's verdict_bytes LE convention) —
  but VERIFY against the actual ctor serialization in the proven harness before committing (read-the-source rule).

### Native KAS handling — REQUIRED, "gas-tank not vault"
- The wallet MUST hold + spend native KAS: every phoque action is a Kaspa tx that costs KAS (fee floors: ~700k
  list/split, ~900k claim, ~1.3M merge), and a buyer's claim PAYS KAS to the seller. Phoques don't move without
  KAS moving. This is forced, not optional.
- POSTURE (not a protocol-enforceable limit — Kaspa has no per-address cap): treat the wallet's KAS as a GAS TANK,
  sized for fees + headroom, presented as "top up for fees," with savings steered to the user's MAIN Kaspa wallet.
  RATIONALE is risk-surface: a browser key WILL sometimes be lost; a lost gas tank is annoying, a lost vault is
  ruinous. Same hold-your-own-keys logic applied to sizing.

### One token mechanism, KAS layer kept separable — DECIDED
- The wallet speaks exactly ONE token type: BitPhoque's own COVENANT token (the KCC-20-style native-L1 pattern —
  built from `kcc20.sil`, copied verbatim as `bitphoque.sil`; CONFIRM the "KCC-20" name against that file in-tree,
  this was recalled mid-session, not re-verified). It is NOT KRC-20 (a different, indexer/inscription-based
  mechanism the project deliberately avoids). Supporting KRC-20 too = a SECOND token stack = PARKED as a separate
  later question; verify current KRC-20 wallet tooling before ever committing (don't decide by assumption).
- BUILD the narrow BitPhoque wallet (route to a FUNCTIONAL system), but architect the KAS layer as a clean,
  SEPARABLE primitive ("send a funded Kaspa tx") with parcel logic sitting ON TOP rather than tangled in. The
  gas-tank BitPhoque wallet and a full general Kaspa wallet share ~80% of their plumbing — keeping them separable
  means a general wallet stays REACHABLE later without being built now (and without foreclosing it). A general
  covenant-token wallet remains "its own later project, arguably THE product" per the sequencing section above.

### Screen flow (wireframed this session)
Seven screens: (1) Landing — seal/joke + two buttons (Create / Restore) + the honest "you hold your own keys"
line. (2) Create → seed reveal → confirm (un-skippable). (3) RESTORE — the cliff: the seed restores the KEY but
NOT knowledge of which parcels exist, so it must immediately pivot to "reconnect your parcels," offering BOTH "find
my parcels (indexer)" and "import receipts manually" (Option-D made literal). (4) Home/balance — total + per-parcel
(held / for-sale), re-verified on-chain each open (why the relay matters). (5) RECEIVE — inverted from a normal
wallet: top half = "my identity (share to receive)," bottom half = "import a receipt (paste/scan)"; the parcel only
becomes real on import. (6) SEND — not done at tx-confirm but at receipt-handoff: surfaces the descriptor as "now
deliver this to the recipient." (7) Sell/list — set price → owner-signed listing (claim/buy likely lives in the
marketplace app, a seam between the two apps).
- KEY INSIGHT the wireframe made obvious: the three HARD screens (Restore, Receive, Send) are hard for the SAME
  structural reason — the amount-derived P2SH leaks through — not from bad design. The INDEXER is exactly what turns
  those three back into normal-feeling screens. Descriptor path = correct + self-erecting; indexer = usable by
  non-technical people. Both needed for the stated target.

### Two real unlocks underneath all the UI (both eventually NEED a synced node to test)
1. **A thin read/broadcast RELAY** so a browser can reach the chain over HTTPS (a browser can't run kaspad / speak
   the CLI's gRPC). Allowed without breaking the lens IFF it only RELAYS (reads public chain state, rebroadcasts
   signed txs) and never holds keys or decides truth — a postal service, not a bank. On the critical path for ANY
   browser wallet.
2. **Tx-building ported CLIENT-SIDE (Rust→WASM)** so signing happens in the browser, never on a server. The engine
   is proven Rust BUT proven as CLI harnesses calling a gRPC client, not as browser-side signing — porting it
   byte-identical is real work and the place an ABI/stale-file bug would hurt most. "Prove the smallest piece
   first": get ONE tx type building + signing in WASM and verifying against the chain before porting the rest.

### Build order implied (all the design above is node-INDEPENDENT and is now captured; the rest is gated on the node)
Descriptor format (keystone) → CLI gains `send`/`receive` (emit/import + verify a descriptor) on top of the proven
engine → marketplace as the optional indexer → web app UI pointed at node/indexer → native wraps. The relay +
client-side signing are the true unlock that everything visual sits on. NEXT NODE-GATED STEP is still the claim
live dry-run (see resume point) — none of this wallet design changes that gate.

---

## TN10 ground facts
- Node: **TestnetNode at 192.168.0.104** — release **kaspad 2.0.1** (post-Toccata; the Toccata guide's recommended
  floor is 2.0.1+). gRPC **16210** all-ifaces (this is what the CLI's GrpcClient uses — `--rpclisten=0.0.0.0:16210`),
  wRPC borsh **17210** all-ifaces (`--rpclisten-borsh=0.0.0.0:17210`). **CORRECTION (do not repeat the old error):
  16211 is the P2P port, NOT gRPC.** An earlier note here said "gRPC 16211" — WRONG; the CLI pointing at 16211 got
  `Unimplemented` (P2P answering a gRPC request). gRPC default on testnet is 16210; the launch .bat MUST include
  `--rpclisten=0.0.0.0:16210` or gRPC falls back to 127.0.0.1-only (unreachable off-box). Launch .bat:
  `C:\kaspatestnet10\start-tn10.bat`; datadir under user **`Karrrl Kaspa Node`** at
  `...\AppData\Local\rusty-kaspa\kaspa-testnet-10\datadir`. The old PsychoNode @ 192.168.0.206 is RETIRED for
  testnet — that hardware now runs MainnetNode (public p2p 16111 forwarded; separate infra, out of scope for TN10).
  **TOCCATA NODE REBUILD (2026-06-22):** the TN10 node kept wedging at utxoindex/consensus-init — root cause was a
  mainnet-line **kaspad 2.0.0** binary running on a **pre-Toccata datadir** (consensus store last written June 9;
  Toccata activated on TN10 ~June 20). Both .104 and .206 had pre-Toccata datadirs (both useless now). Fix: kaspad
  2.0.0→**2.0.1**, wiped to a FRESH datadir (old one moved to `datadir_pretoccata_old`), clean post-Toccata IBD.
  LESSON: a wedge at the index step on a node that "was synced" can be a stale/incompatible DATABASE, not a code or
  index bug — check the binary version and the consensus-store date against the last hardfork before fighting it.
  kaspad must always be stopped with Ctrl+C and allowed to FLUSH (watch for clean exit); a force-kill mid-flush
  corrupts the utxoindex (rename-and-rebuild recovery).
- Funding addr (reused for both genesis runs): `kaspatest:qrwzyuc8lkvdsxgww53n7rsjetsqhh6p9hmy8vrvqpdun5r7ysevufmglhgl4`
  (key in bitphoque_owner.key; re-faucet-funded 100k TKAS for the v2 genesis).
- rusty-kaspa clone: C:\kaspa-tn12\rusty-kaspa (tag tn10-toc3); silverscript: C:\kaspa-tn12\silverscript.
- Faucet: min 1000 TKAS per request, 24h cap 100k. Fund wallets 1000 at a time so the cap is never a constraint.
- Fee floors learned: genesis ~167k (use 200k); transfer ~378,600 (use 400k); v2 claim ~654,600 (use 700k);
  v3 list/split ~659k (use 700k); v3 2-input claim 857,000 (use 900k). v3 PARTIAL-LIST (1-in/2-out
  owner-signed) clears at 700k like the split (one script reveal, two outputs).

---

## RESUME POINT — CLI feature-complete (claim committed); next is claim's LIVE dry-run once TN10 IBD finishes

**Immediate next action (when the TN10 node @ .104 finishes its fresh post-Toccata IBD):** fix the CLI `NODE` const
to `192.168.0.104:16210` (gRPC — NOT 16211), rebuild (copy canonical → `C:\kaspa-tn12\silverscript\silverscript-lang\src\bin\`,
`cargo build --bin bitphoque_cli`), then run `balance` against buyer1 (`8c02c20a...`) to confirm the node serves a
live by-address lookup. Then the headline: a LIVE `claim` dry-run (it VM-verifies the covenant input against a real
for-sale parcel — the one proof claim hasn't yet gotten). If PASS, claim is fully validated end-to-end like the
other subcommands. Then: extract shared lib → thin marketplace app → mainnet checklist.

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
- [x] **Split/merge generalization for the sale** (MERGE N→1) — DONE on-chain (`519063be...`). Two
      not-for-sale same-owner parcels (20033+5268) → one 25301 parcel, owner-signed, leader/delegate
      ABI, conservation enforced, both inputs confirmed spent. The two-signed-covenant-input finding
      (both sigs at leader index 0) is recorded above. (Partial-balance listing — the sale-specific
      split/merge — now also DONE on-chain, `4b0c8187...`, see below.)
- [x] **Generalized inspect tool** — DONE (`bitphoque_inspect_v3.rs`). Compiles any v3 parcel P2SH from
      CLI params, queries with retry for the index lag, optional outpoint filter. Read-only, pubkey-only
      (stream-safe). Supersedes the hardcoded rothschild inspect for v3.
- [x] **wallet/CLI build** — DONE in code. `bitphoque_cli.rs` feature-complete (import / balance / partial-list /
      list / split / merge / claim), all built + committed (claim `78c14d9`). Option-C track-forward wallet.json.
      ONLY remaining: claim's LIVE on-chain dry-run (blocked on TN10 IBD — see resume point). Still open beyond
      that: extract shared lib → thin marketplace app; original-token owner quirk; AUCTION future build; mainnet
      checklist; make public.
- [x] **Split/merge for the sale (list a PARTIAL balance)** — DONE on-chain (`4b0c8187...`). One owner-
      signed tx lists 5000 of buyer1's 25301 merge parcel @ 500000 sompi (out0, for-sale) + keeps 20301
      (out1, not-for-sale change), same owner — collapses the old two-step split-then-list into one tx.
      Harness `rust-examples/bitphoque_partial_list_v3.rs`. Confirmed: input 519063be...:0 spent;
      for-sale parcel live at 4b0c8187...:0 (163036283150 sompi). checkSaleState allows it (no input
      for-sale -> for-sale OUTPUT permitted); proportional value split, out1 absorbs remainder, dust-
      floor guard.
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
      **FINDING (2026-06-22 secret sweep):** an actual 64-char hex TN10 privkey is committed in `docs/bitphoque_status.md`
      (the original-token recipient key, `1c14a1de...`, in the section-2 transfer note). It is a TESTNET throwaway
      (no value, quarantined to TN10), so not an emergency — but per the baseline "private keys never belong in a
      tracked file." Before going public: scrub the hex from the doc (redact like the other key entries) AND rewrite
      it out of git history (git-filter-repo / BFG), since history is permanent. Do this as a deliberate pre-public
      step, not casually. The rest of the sweep was clean (only docs/comments ABOUT keys).

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
