# BitPhoque — History (permanent record)

*The DURABLE ARCHIVE for BitPhoque. This holds the settled, never-changing record — every confirmed
on-chain transaction (with txids) and solved war stories — that the live status doc (`bitphoque\_status.md`)
should NOT carry, because a session-start doesn't read them; they're consulted ON DEMAND when a specific
question comes up ("what txid was the merge?", "how was the 520-byte wall solved?").*

***Read order:*** <i>start a fresh session from `bitphoque\_status.md` (where we are + what's next) and
`operating\_baseline.md` (how we work). Come HERE only to look up a specific historical fact. The split rule:
things that CHANGE live in status; things that are DONE and permanent live here. A confirmed txid is the most
"done" thing there is — it will never need editing.</i>

*NOTE: consensus rules / fee-floor derivations / node-and-indexing quirks were DELIBERATELY KEPT in the status
doc (they're broadcast lessons still actively consulted when building txs), not moved here. This file is the
on-chain transaction ledger + solved-problem war stories only.*

\---

## INDEX

* **On-chain history (TN10), sections 1–12** — every live tx in order: genesis, transfer, v2 sale lifecycle,
v3 genesis/split/2×2 sale/resale/transfer-split/combined-2×2-claim/merge, plus the generalized inspect tool.
* **The 520-byte wall — SOLVED** — the `ElementExceedsMaxSize(1540, 520)` war story (a builder default, not a
consensus rule).

\---

## ON-CHAIN HISTORY (TN10) — what's live, in order

### 1\. GENESIS (original token) — LIVE

* tx: `f8d6d45a37f6049872bb12d55359a939e6677e6848ad6135a9e1fd2cbbb00875`
* covenant\_id: `69262e04b63d1ea1008fd33f8aff38c08780056b40111d0db01a306ef11a766c`
* owner pubkey (x-only): `dc227307fd98d8190e75233f0e12cae00bdf412df643b06c005bc9d07e2432ce`
* genesis covenant value: 9,999,999,800,000 sompi (100k TKAS minus 200k fee)
* real redeem script: 1540 bytes (fine on-chain — covenant element cap is 1,000,000).
* Token P2SH: `kaspatest:prrejfkltxlctd0htkma7p50l45tc7fy242vf4g475tc2y0k5rlggne36eqz6`
* The whole supply (42,069) minted into one genesis UTXO; "holding" = controlling the owner key.

### 2\. TRANSFER (handoff) — LIVE

* handoff tx (ACCEPTED): `907f738332268109aea5bacc0670fb59c20af76f17e55c173ca83aa9416a4da1`
* genesis cov UTXO `f8d6d45a...0875:0` SPENT; genesis token P2SH now EMPTY (inspect tool confirmed).
* recipient (owns all 42069 of the ORIGINAL token): pubkey
`6687942178295b30101e84115f8caf56dbcc0a20008c352587ce4e76da3718b0`
privkey `1c14a1de…REDACTED` (TN10 practice; throwaway — full hex scrubbed per baseline "no private keys in
tracked files"; original-token recipient key, quarantined to TN10, no value. Still present in git HISTORY of
the old status doc — a `git-filter-repo`/BFG rewrite is the pre-public step, see status doc's make-public note).
* harness: `rust-examples/bitphoque\_transfer.rs`. Local VM verify gates `--go`.

### 3\. SALE (v2 sale-enabled token) — LIVE, full lifecycle proven on-chain

A SECOND deployment (v2 contract) to prove the fixed-price sale. Separate from the original token.

* v2 contract: `covenants/bitphoque\_sale\_v2.sil` (checkSaleState listing/claim separation). Script 1969 bytes.
* v2 covenant\_id: `9a30cef719c6ac08eaf94f701dba43136f3a6837b4d24c419610507250367bf3`
* v2 token OWNER (the SELLER): `50a1553911677db039333592faf1c0757a4e18999fd9be68da0f1b1ab60cb11b`
(privkey in gitignored `bitphoque\_v2\_owner.key`). Payout addr: `kaspatest:qpg2z4fez9nhmvpexv6e97h3cp6h5nscnx0an0ngmg83kx4kpjc3k6s5w25c5`
* BUYER key: `261cbf44ce71a1501fe7fc2d6191ccd657c0fd170297b2b871533ced62c54420`
(privkey in gitignored `bitphoque\_buyer.key`). Faucet-funded separately (genuinely distinct parties).
* **(3a) GENESIS v2** tx: `b1086a59c163c4fbd8f9c333fc419f517c695e41f8989ddb23d7e945fbcdc6e2` (42069, not-for-sale).
harness `rust-examples/bitphoque\_sale\_genesis.rs` (reads `bitphoque\_sale\_compiled.json`).
* **(3b) LISTING** tx (ACCEPTED): `cad1220f61494e70f6a15b3316ea12d4f218c0929a36bd0fc86c739d68835ea0`
Owner-signed; flipped the genesis UTXO to forSale=true at price 500,000 sompi. New for-sale UTXO `cad1220f...:0`.
harness `rust-examples/bitphoque\_sale\_list.rs`.
* **(3c) CLAIM** tx (ACCEPTED): `592a0d5b54469e78d4b43af4298a6ec063b78cf3dd7d1fc708af688920934eb7`
Buyer paid 100,000,000 sompi to the seller and received the 42069 phoques — NO owner sig on the covenant
input (payment-authorized). Phoques now in a new covenant UTXO owned by the BUYER key, not-for-sale.
harness `rust-examples/bitphoque\_sale\_claim.rs` (2-input: unsigned covenant input 0 + buyer-signed plain input 1).

### 4\. SALE v3 (fanout-safe, the F1 fix) — LIVE genesis

First-ever v3 broadcast. v3 fixes the F1 double-claim hole (for-sale inputs must use
strictly-increasing payment-output indices). Supersedes the holed v2 as the live sale token.

* v3 contract: `covenants/bitphoque\_sale\_v3.sil`. Compiled script 2793 bytes.
* v3 covenant\_id: `3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458`
* v3 genesis tx (ACCEPTED + confirmed): `f1e53e2ff1a7c58145b4ecaca0d06b14ff7f1fba2083b95546a861cd87640ddf`
* genesis covenant UTXO `f1e53e2f...:0`, value 3,299,999,800,000 sompi, 42069 phoques, not-for-sale, maxCovIns/Outs=3/3.
* owner = same v2 seller pubkey `50a1...`; funding input `fc91589116b0de5a4f9f019cb3520564ec9d5d014051f41bec8ec0c8c42f2682:0` spent (confirmed empty).
* harness `rust-examples/bitphoque\_sale\_genesis\_v3.rs` (reads `bitphoque\_sale\_compiled\_v3.json`).
* NOTE: funded with 33,000 TKAS (faucet 24hr cap 100k; conserved the rest for list/claim funding).

### 5\. SPLIT v3 (1 -> 2, prerequisite for 2x2) — LIVE

The genesis minted all 42069 into one UTXO; a 2x2 sale needs two. Owner-signed 1-input/2-output
transfer, both outputs not-for-sale, same owner. First on-chain MULTI-OUTPUT transfer on v3
(proves conservation across two outputs on a real node).

* split tx (ACCEPTED): `5db053c41fe1865d89fb2c70dfb890b6c5a10bef80e053fcd302d5d47b3aa8e9`
* out 0 `5db053c4...:0` = 21034 phoques, \~16,499.995 TKAS, owner 50a1..., not-for-sale
* out 1 `5db053c4...:1` = 21035 phoques, \~16,499.995 TKAS, owner 50a1..., not-for-sale
* harness `rust-examples/bitphoque\_sale\_split\_v3.rs` (silverscript bin). Caps 3/3 (matches genesis).
* NOTE: each output sits at a DIFFERENT P2SH (amount is a ctor arg -> bakes into script). The list
harness must compile with the right amount to find each UTXO.

### 6\. SALE v3 — 2×2 PRIMARY SALE (two separate claims) — LIVE

The two split UTXOs (21034, 21035) listed and claimed independently by two DISTINCT buyers.
Proves the v3 claim path on a real node and the "sell over time to successive buyers" use case.

* buyer0 = `261cbf44ce71a1501fe7fc2d6191ccd657c0fd170297b2b871533ced62c54420`
(key `bitphoque\_buyer.key`; addr `kaspatest:qqnpe06yeec6z5qlul7z6cv3ent90s8azupf0v4cw9fnemtzc4zzqa8xadms9`)
* buyer1 = `8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936`
(key `bitphoque\_buyer1.key`; addr `kaspatest:qzxq9ss2v7gpz2x6hymhhjeajuujjm5z4skeua3gvdw67clx84ynvxfwgm0j4`)
GENERATED + faucet-funded this session (faucet tx `3e7b89c1...`, 1000 TKAS; faucet min is 1000).
* **(6a) LISTING 21034** tx: `cebf75b5fafbdad3c38505fdeec4b7db3542bd9b88a448cbab219e6f4caa2443`
(owner `50a1...` signed; price 500000; for-sale UTXO `cebf75b5...:0`). Spent split `5db053c4...:0`.
* **(6b) LISTING 21035** tx: `55542a993f104dae1d304c95420a8c14f4805a7b6c37909d48a12bbfc361c59f`
(owner `50a1...` signed; price 500000; for-sale UTXO `55542a99...:0`). Spent split `5db053c4...:1`.
* **(6c) CLAIM 21034 by buyer0** tx: `3265e0635217c37acc32247cbdd34b83072b4e071d9d644b5f4ea6ceedd1f085`
(buyer0 paid 100M to seller `50a1...`, received 21034; not-for-sale UTXO `3265e063...:0`).
* **(6d) CLAIM 21035 by buyer1** tx: `9ffe84b8a6d56b691c3748b82aaccc35b2b440857ebca650b46c05bba3f6ed6a`
(buyer1 paid 100M to seller `50a1...`, received 21035; not-for-sale UTXO `9ffe84b8...:0`).
* harnesses: `rust-examples/bitphoque\_sale\_list\_v3.rs` + `rust-examples/bitphoque\_sale\_claim\_v3.rs`.
* NOTE: these are TWO SEPARATE claim txs (one for-sale input each). They prove single claims on v3.
They do NOT yet exercise the strict-increasing F1 accumulator — that needs the COMBINED 2×2 tx
(two for-sale inputs in ONE tx), which is the remaining headline open item.

### 7\. SALE v3 — SECONDARY-MARKET RESALE (buyer0 → buyer1) — LIVE

A peer-to-peer resale with Kyle (genesis owner) NOT a party. Proves the token CIRCULATES: a holder
who BOUGHT phoques can list and sell them with the identical tools, because the protocol treats every
holder as an equal peer (no privileged seller / no coordinator). This is the on-chain answer to "how do
buyers sell to other buyers."

* **(7a) buyer0 RESALE LISTING** tx: `d24437b8bddd37942076fbb608f9f05d21b4d094d17e2cb6fd1a046322a5c088`
buyer0 signed (with `bitphoque\_buyer.key`) to list their own 21034 at buyer0's OWN price 750000
(a reseller sets their own price). Spent buyer0's not-for-sale UTXO `3265e063...:0`. For-sale UTXO
`d24437b8...:0` at P2SH `kaspatest:prqhnhtk3vraw2fffvn3ezvmhaxqtv8nfxrfwm9v0wn9nu58kckxgyqwxq5xu`.
* **(7b) buyer1 CLAIMS buyer0's listing** tx: `1635fd2858223314ef4f0b350c63b2627fb18a0d4836aed69a2c4125b391b720`
buyer1 paid 100M to **buyer0** (not Kyle), received 21034. The claim harness's party-agnostic seller
arg = buyer0's pubkey `261cbf44...`. No Kyle signature, no Kyle involvement anywhere in this hop.
* CHAIN OF CUSTODY (21034 parcel): Kyle `50a1...` → buyer0 (6c) → buyer0 lists (7a) → buyer1 (7b).
* END STATE: buyer1 holds the whole supply (21034 from buyer0 + 21035 from Kyle = 42069). buyer0 net:
paid 100M to acquire, received 100M reselling (TN10 practice values).

### 8\. TRANSFER-SPLIT → realistic 3-holder distribution — LIVE

Generalized the self-split into a TWO-OWNER transfer-split (each output gets its own owner pubkey),
then seeded a realistic uneven distribution across three holders for future wallet/marketplace work.
NEW keys this session (generated + saved gitignored; buyer2/3 faucet-funded 1000 TKAS plain each):

* buyer2 = `5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161`
(key `bitphoque\_buyer2.key`; addr `kaspatest:qpwn8z49f085m92eg8lchnwsdxe082saadqzxnw88njlfavp4hckzcys2w2lh`)
* buyer3 = `267496ce01f2a35653491c61b56b23e6b7dbe777a02b146286ac3635ff36ab61`
(key `bitphoque\_buyer3.key`; addr `kaspatest:qqn8f9kwq8e2x4jnfywxrdtty0nt0kl8w7szk9rzs6krvd0lx64kzkhgpw6d5`)
* **(8a) split A** `81caee43...` : buyer1's 21034 → 1000 (buyer2, `81caee43...:0`) + 20034 (buyer1, `...:1`)
* **(8b) split B** `05160528...` : buyer1's 21035 → 10500 (buyer3, `05160528...:0`) + 10535 (buyer1, `...:1`)
* **(8c) split** `4f084e26...` : buyer1's 20034 → 1 (listing parcel, `...:0`) + 20033 (buyer1, `...:1`)
* **(8d) split** `0434801c...` : buyer1's 10535 → 5267 (listing parcel, `...:0`) + 5268 (buyer1, `...:1`)
* harness `rust-examples/bitphoque\_sale\_split\_v3.rs` is the self-split; the two-owner generalization is
`rust-examples/bitphoque\_split\_transfer\_v3.rs` (CLI below). KEY FINDING: a split to a DIFFERENT owner
is just a normal transfer — the contract constrains AMOUNTS (conservation), not output ownership, so
the input owner can hand parcels to anyone. Verified at VM + on-chain.

### 9\. COMBINED 2×2 CLAIM — THE ON-CHAIN F1-FIX PROOF — LIVE

The headline. buyer1 listed TWO of its parcels (same seller): `4943a956...:0` (1 phoque) and
`5aab21f6...:0` (5267 phoque), both price 500000. Then ONE tx claimed BOTH:

* **2×2 claim tx (ACCEPTED):** `40cbf8659db6e0d17465c0916da4581557483e8a5e3d204e91e12d085b79d80d`
* input 0 = for-sale `4943a956...:0` (1 phoque, seller buyer1), UNSIGNED, **witness → output 2**
* input 1 = for-sale `5aab21f6...:0` (5267, seller buyer1), UNSIGNED, **witness → output 3** (3 > 2)
* input 2 = buyer2 plain funding (buyer2-signed); input 3 = buyer3 plain funding (buyer3-signed)
* output 0 = 1 phoque → buyer2; output 1 = 5267 → buyer3
* output 2 = 100M → seller buyer1 (buyer2 pays); output 3 = 100M → seller buyer1 (buyer3 pays)
* outputs 4,5 = change to buyer2 / buyer3
* **What it proves:** two for-sale inputs from the SAME seller, in ONE tx, forced onto STRICTLY-
INCREASING DISTINCT payment outputs. The v2 F1 hole (witnesses=\[k,k] → two parcels, one payment)
is structurally impossible here and confirmed on a real node. Same-seller is the strongest F1 test:
cross-seller was always caught by the wrong-payee check (F2); same-seller was the actual hole.
* **Also proves atomic multi-party settlement:** two distinct buyers each fund their own payment in
one atomic tx — the whole tx commits or none of it does; neither buyer can be rugged.
* harness `rust-examples/bitphoque\_sale\_claim\_2x2\_v3.rs` (hardcoded consts for this one-shot proof,
not the general CLI). fee 1,500,000 cleared on first try (no floor correction needed).

FINAL DISTRIBUTION after the 2×2 (whole supply across 3 holders, all by genuine on-chain sales/splits):

* buyer1: 20033 + 5268 = 25,301   |   buyer2: 1000 + 1 = 1,001   |   buyer3: 10500 + 5267 = 15,767
* total 42,069 ✓

### 10\. MERGE 2→1 (consolidate parcels) — LIVE

The inverse of the split: TWO not-for-sale, SAME-owner covenant UTXOs collapsed into ONE. First
on-chain covenant MERGE for BitPhoque. buyer1 held two parcels (20033 + 5268) at two different
P2SHes; merged into one 25301 parcel. Owner-SIGNED on BOTH inputs (both not-for-sale → checkAuth's
checkSig branch). Conservation enforced (totalIn == totalOut). 2-covenant-input tx → leader/delegate
ABI (same machinery as the 2×2 claim, but signed not payment-authorized).

* **merge tx (ACCEPTED):** `519063be8d37e7768ba93e54c517ced3fb32eba89b7194b3c15bf0e72fe0f15c`
* input 0 = `4f084e26...:1` (20033, buyer1), owner-SIGNED, LEADER
* input 1 = `0434801c...:1` (5268, buyer1), owner-SIGNED, DELEGATE
* output 0 = `519063be...:0` = 25301 phoques, value 824,996,900,000 sompi (= both inputs − 1.3M fee),
buyer1, not-for-sale
* BOTH old parcels confirmed SPENT on-chain (inspect\_v3 returned empty for 20033 and 5268 after merge).
* harness `rust-examples/bitphoque\_sale\_merge\_v3.rs` (CLI: `<amount0> <amount1> <owner\_keyfile> \[--go]`).
* **THE TWO-SIGNED-COVENANT-INPUT FINDING** (the bug that cost one VerifyError + the fix): both owner
sigs are computed against the LEADER input's sighash (INDEX 0), NOT against each input's own index.
The covenant verifies ALL input sigs from the leader's position. Signing sig1 against input 1 gave a
different sighash → VerifyError. Fixed by signing both against input 0, per the proven kcc20\_tests.rs
merge pattern (which signs both at index 0 and uses witness\_array \[0,1]). This is the FIRST tx that
signs two covenant inputs — the 2×2 claim's covenant inputs were UNSIGNED (payment-authorized), so it
never surfaced there. Carry this forward to the CLI/wallet: any owner-signed multi-covenant-input tx
(merge, multi-parcel transfer) signs every covenant input at the leader index.

### 11\. CLI CLAIM (first claim through the CLI) — LIVE, CONFIRMED MINED

The first claim broadcast via `bitphoque\_cli claim` (prior claims were standalone harnesses). buyer2
paid buyer1 and claimed 5000 phoques from the for-sale parcel `4b0c8187...:0`. Node-ACCEPTED into
mempool 2026-06-23 while TN10 was mid-IBD; CONFIRMED MINED 2026-06-25 once the relocated Big-Daddy
node finished syncing (parcel `4b0c8187...:0` and its 20301 change input `4b0c8187...:1` both read
SPENT/NOT-FOUND on the synced node — the claim consumed them).

* **claim tx (ACCEPTED + MINED):** `28a90bc9a5ae922837e05f10952be500be289709cca8cf55461d6fd6ed5fec35`
* buyer2 → buyer1 payment 100,000,000 sompi (1 TKAS), seller `8c02c20a...`
* claimed 5000 phoques from for-sale parcel `4b0c8187...:0`, fee 900k, change \~99.8 TKAS
* LOCAL VM VERIFY passed before broadcast; the 100M payment outpoint later observed at buyer1's P2PK
address (`28a90bc9...:1`, 100000000 sompi) confirming the payment leg landed.

### 12\. WHOLE-PARCEL SEND (the last untested transfer path) — LIVE, MINED + ROUND-TRIPPED

The whole-parcel `send` path (1-input → 1-output, no change, recipient owns out0) — VM-proven in a
prior session but never broadcast — fired live on the synced Big-Daddy node. buyer1 sent its entire
16301 not-for-sale parcel to buyer2. Confirmed MINED by the `receive` round-trip: `receive` decoded
the send-emitted descriptor and confirmed the new parcel on-chain (in the UTXO set, not just mempool).
This closes the send/receive surface completely — partial (proven earlier) + whole, both directions.

* **send tx (ACCEPTED + MINED):** `b52e19c6c629655565f4fa98b016fdc36199841db515e55007cb6643f18435ba`
* spent cov UTXO `e45d9c2b...:1` (16301, buyer1, not-for-sale), value 531,529,668,714 sompi
* output 0 = `b52e19c6...:0` = 16301 phoques to buyer2, value 531,528,968,714 sompi (= input − 700k fee)
* send emitted a base58check recipient descriptor; `receive` (as buyer2) decoded it, gated covenant\_id,
recomputed P2SH locally, and CONFIRMED `b52e19c6...:0` on-chain (value matched exactly), recorded in
wallet.json. Full whole-parcel send→receive loop proven live.

### Generalized inspect tool — `rust-examples/bitphoque\_inspect\_v3.rs` (BUILT this session)

Read-only. Compiles ANY v3 parcel's P2SH in-process from CLI params (owner pubkey + amount + forSale +
price; same ctor shape as the list harness) and queries the node. Takes a PUBKEY (never a keyfile) —
stream-safe, no key material. CLI: `<owner\_pubkey\_hex> <amount> <forsale:0|1> <price> \[--outpoint txid:index] \[--retries N]`. Supersedes the hardcoded-address rothschild `bitphoque\_inspect.rs` for v3
parcels (that one still works for the original v1 token). Used 3× this session to confirm the merge
(parcels live pre-merge, 25301 result present, both inputs spent). Silverscript-clone bin (needs the
compiler API to build the P2SH).

\---

## THE 520-BYTE WALL — SOLVED (a builder default, not a consensus rule)

The first transfer panicked `ElementExceedsMaxSize(1540, 520)` pushing the redeem script. Reading the txscript
engine source (`crypto/txscript/src/lib.rs:80-81,143` + `data\_stack.rs:331`):

* `MAX\_SCRIPT\_ELEMENT\_SIZE\_PRE\_TOCCATA = 520`; `...\_POST\_TOCCATA = 1\_000\_000`.
* `max\_script\_element\_size(covenants\_enabled)` returns 1,000,000 when covenants are enabled.
* The 520 cap was only the conservative default of `ScriptBuilder::new()` (covenants\_enabled:false).
* FIX (one line): build the redeem push with
`ScriptBuilder::with\_flags(EngineFlags { covenants\_enabled: true, sigop\_script\_units: 0.into() })`.
* CONSEQUENCE: NO contract surgery, NO regenesis, NO chunked reveal. Verify-before-gutting paid off hard —
we nearly regenesis'd a leaner contract to beat a non-limit. (Abandoned shrink experiments: git 961959b.)

