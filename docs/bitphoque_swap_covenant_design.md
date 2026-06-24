# BitPhoque — Swap Covenant Design Note (the trustless-sale mechanism)

_Status: DESIGN ONLY — not built. This is the spec to start from next session. It is the
answer to "how do I sell real BitPhoques to a stranger without a middleman." Same stack,
same discipline as the transfer harness (which is DONE + proven on TN10): write it, read
the source for what enforces, local VM verify, REJECTION TESTS for every cheat, TN10 before
mainnet. Read alongside bitphoque_status.md and bitphoque_mainnet_checklist.md._

---

## BUILD PROGRESS (updated this session)

The FOR-SALE-STATE model is now PARTLY BUILT and PROVEN at the VM level (single-claim case):
- Q1 RESOLVED (compile probe): covenant-bound (binding=cov) entrypoints CAN read tx.outputs[...]. Compose confirmed.
- Q2 RESOLVED (compile probe): forSale(bool)+price(int) state fields compile. New state layout {start:1, len:57} (was 46).
- CONTRACT v1 written + compiles: covenants/bitphoque_sale_v1.sil (git 5df836a). checkAuth branches per-input on
  prevStates[i].forSale: not-for-sale= sig; for-sale= tx.outputs[witnesses[i]] pays  to
  seller's P2PK lock. checkSaleReset forces every new output forSale=false (anti-double-claim).
- SALE GATE PROVEN (rust-examples/bitphoque_sale_reject_tests.rs, git 5df836a): CONTROL passes; all 6 cheats
  REJECTED at the VM (underpay / wrong-payee / no-payment / claim-unlisted / stays-for-sale / output-match-abuse).

STILL OPEN (next session): generalize beyond SINGLE-CLAIM (multi-input/output sale fanout + harder
  output-matching attacks: coincidental extra payment, cross-referenced witness indices); confirm sig-shape
  games can't block/alter a for-sale claim; then ON-CHAIN (re-encounters v1-mass/ComputeBudget/fee rules from
  the transfer; needs a broadcast harness + FRESH GENESIS on the new contract since layout/script changed);
  the listing path (owner-signed flag flip) still needs its own harness + reject test (flip without sig MUST fail).

---

## Why a swap covenant at all (the gap)

The BitPhoque covenant guarantees token MATH — conservation (no inflation) and ownership
(only the owner key can move a UTXO). It guarantees NOTHING about payment. A plain transfer
moves phoques but has no idea whether the sender got paid. So a plain transfer cannot be a
"sale" — the buyer could receive phoques and simply never pay.

A SWAP COVENANT closes that: it makes token-release and KAS-payment ATOMIC. Either both legs
settle in ONE transaction or the transaction is rejected by the covenant. No escrow, no
trusted third party, no indexer. This is the BitPhoque thesis (neither party can cheat the
other) applied to the trade layer — the "prime directive at the trade layer."

NOTE on ecosystem context (checked June 2026): there is NO native Kaspa DEX yet (months out,
post-Toccata). The wallets that handle "tokens" (KasWare, Tangem) handle INSCRIPTION-based
KRC-20 via the Kasplex indexer — a DIFFERENT token model that BitPhoque deliberately rejected.
None of them see or spend a covenant-token UTXO. So for BitPhoque, the trustless sale is a
BUILD (this covenant), not a buy (some wallet feature). Owner-key storage = KasSigner
(air-gapped). The "wallet" that spends a BitPhoque is our own harness/swap tooling.

---

## THE FOR-SALE-STATE MODEL (preferred sale architecture — per-UTXO flag)

_This is the cleaner evolution of the swap design and the RECOMMENDED direction. It resolves the
"Pattern A vs B" authorization muddle by splitting the two authorizations cleanly via a per-UTXO
sale flag. Decided: per-UTXO sale state (a UTXO/lot is for-sale), NOT per-phoque ids. Per-phoque
ids would make BitPhoque an NFT collection (new asset, contract replaced, 42069 distinct items) —
explicitly NOT what we want; BitPhoque stays a FUNGIBLE token. Per-UTXO state gives ~all the
benefit (seller picks what/when is for sale) while EXTENDING the proven contract, not replacing it._

### The idea
Each BitPhoque UTXO carries sale state: a `forSale` flag and a `price`. The seller flips a UTXO
between not-for-sale and for-sale-at-price-P. A for-sale UTXO can be CLAIMED by any buyer who pays.
Seller controls exactly what is on the market, at what price, and when — all by flipping state on
specific UTXOs, while the rest of their phoques sit safe in normal (sig-required) state.

### Why this is SAFER than "any UTXO claimable if paid"
It cleanly separates two authorizations:
- **Listing / re-pricing / de-listing** (flipping the flag, setting price, pulling off market):
  OWNER-AUTHORIZED — requires the seller's signature, exactly like a normal transfer.
- **Claiming a for-sale UTXO**: PAYMENT-AUTHORIZED — no owner sig; release is gated by payment.

So the dangerous unsigned-spend path applies ONLY to UTXOs the seller DELIBERATELY marked for sale
at a price they set. A bug in the payment check can only ever drain UTXOs the seller already chose
to expose — never their whole holdings. This dramatically shrinks the attack surface vs. a blanket
"anything is claimable" model.

### Two spend modes (gated by the forSale flag)
- **Normal transfer** (forSale = false): owner sig required (existing `checkSigs` path). Unchanged.
- **Sale claim** (forSale = true): payment-to-seller required INSTEAD of owner sig; new owner =
  buyer (named by the claimant in the new-state array); conservation still holds; AND the resulting
  UTXO MUST reset forSale = false (now owned by the buyer).

### CRITICAL covenant condition: claim MUST reset the flag
"When the sale happens, that phoque is no longer on the market" is a REQUIRED covenant condition,
not a nicety. The claim tx must produce a new UTXO whose state is forSale=false, owner=buyer. If the
covenant did not enforce this, a claimed phoque could stay flagged for-sale at the old price and be
claimed AGAIN by someone else for a second payment to the OLD seller. Enforce: claimed output's
forSale == false. Add a rejection test: a claim that leaves forSale=true MUST fail.

### What changes in the contract (and the consequences)
- State struct gains `forSale` (bool) + `price` (int). Currently state is owner/identifierType/
  amount/isMinter; this adds two fields. State layout (currently {start:1, len:46}) CHANGES.
- New compiled script -> new P2SH -> new covenant_id -> FRESH GENESIS required (same regenesis
  consequence already accepted for any contract edit; the live token is on the current contract).
- `checkAmounts` / `checkMintingTransfer` iterate state — confirm they still hold with the extra
  fields (they key off amount/isMinter, so should be unaffected, but verify).
- The sale-claim path is a NEW entrypoint (or a new mode of transfer) whose authorization is the
  payment check, not checkSigs. This is the airtight-or-bust part (unsigned spend).

### Payment binding (reuses escrow primitives — CONFIRMED expressible)
The claim covenant checks: `tx.outputs[payoutIdx].value >= price` AND
`tx.outputs[payoutIdx].scriptPubKey == <seller's payout lock>`. Both proven expressible by the
example contracts (covenant_escrow.sil: `tx.outputs[0].value == amount`,
`tx.outputs[0].scriptPubKey == byte[](sellerLock)`; ScriptPubKeyP2PK(party) builds the lock).
The seller's payout lock = the seller's own address (the current owner being bought out).

### Rejection tests for the sale-claim path (the whole safety case — unsigned spend!)
- CONTROL (must PASS): claim a for-sale UTXO, pay >= price to seller, new owner = buyer,
  forSale reset to false. Accepted.
- REJECT (must FAIL): claim but pay LESS than price.
- REJECT (must FAIL): claim but pay to the WRONG address (not the seller/old owner).
- REJECT (must FAIL): claim with NO payment output.
- REJECT (must FAIL): claim a NOT-for-sale UTXO via the payment path (forSale=false must force
  the sig path — payment must not authorize spending a non-listed UTXO).
- REJECT (must FAIL): claim leaves forSale = TRUE (would allow double-claim / second payment to old
  seller).
- REJECT (must FAIL): output-matching ambiguity — the "payment" output reused as change.
- MUST HOLD: conservation across the claim (no phoques created/destroyed).
- (Listing path) REJECT: flipping forSale / changing price WITHOUT the owner's signature.

### Open questions to verify-from-source before coding this
- [ ] Can a covenant-bound entrypoint (binding=cov) ALSO read tx.outputs[...] for the payment check,
      and compose with the macro-generated state validation? (escrow is a PLAIN covenant; our
      transfer is covenant-bound — confirm the two compose.)
- [ ] Does adding forSale+price to the state struct compile cleanly and what's the new state layout?
- [ ] How does the claimant name themselves as buyer in the new-state array on an unsigned path?

---

## Two patterns (older framing — superseded by the for-sale-state model above for the sale case)

### Pattern A — single-transaction co-signed swap  [BUILD THIS FIRST]
Buyer and seller co-construct ONE transaction:
- **Inputs:** seller's BitPhoque covenant UTXO + buyer's plain KAS UTXO.
- **Outputs:** (1) phoques -> buyer as a new BitPhoque covenant UTXO (owner = buyer pubkey);
  (2) >= X KAS -> seller's address; (3) optional change back to buyer.
- The existing BitPhoque covenant already enforces the TOKEN leg (conservation + new owner).
- The SWAP CONDITION the covenant must ADD: "this covenant UTXO may be spent ONLY IF the tx
  contains an output paying >= PRICE KAS to SELLER_ADDRESS." That single extra output
  constraint is the whole sale guarantee.
- Both parties sign; both legs settle together or the tx fails. Minimal extension of the
  transfer we already proved — it's the transfer PLUS output-payment enforcement.

WHY FIRST: smallest delta over working code; rejection tests are clean to reason about; no
standing on-chain offer to be griefed.

### Pattern B — offer-and-claim (HTLC-style standing offer)  [LATER]
Seller posts a covenant UTXO that ANYONE can spend IF they pay PRICE to seller in the same
tx. No live coordination — a true on-chain "for sale" listing. More powerful (asynchronous,
no co-signing) but more moving parts and more attack surface (front-running, partial fills,
offer expiry/cancellation). Build only after Pattern A is proven.

---

## Payment-binding / nonce (buyer's instinct) — when it's needed and why

The question: should the buyer's payment include a nonce/hash that IDENTIFIES this payment as
belonging to THIS specific swap, vs. a random unrelated payment? Answer is pattern-dependent,
and the distinction sharpens both patterns:

### Pattern A — NO nonce needed (atomicity already binds it)
In a single co-signed tx, the payment output and the covenant spend are THE SAME TRANSACTION.
The covenant accepts or rejects the whole tx atomically; there is no free-floating payment that
could be matched to the wrong swap. Binding is structural: same tx = same swap. A nonce here
solves a problem the single-tx construction already eliminates. (Still must handle output-MATCHING
ambiguity — see below — but that is index/identifier binding, not a time-decoupled nonce.)

### Pattern B — nonce/identifier is LOAD-BEARING
A standing offer decouples payment from claim in time and identity, which opens real attacks a
nonce/identifier closes:
- **Replay across offers:** two identical offers (same price+address) could let one payment satisfy
  both, or a payment for offer #1 claim offer #2. A unique per-offer identifier forces the
  satisfying payment to reference THAT offer.
- **Coincidental-payment ambiguity (exactly the buyer's point):** without an identifier the covenant
  can only check "did SOME output pay >= X to address Y?" — a random unrelated payment to Y that
  clears the bar could spuriously satisfy it. An identifier the covenant requires distinguishes
  "payment FOR THIS offer" from "a payment that happens to look like one." The nonce binds INTENT,
  which amount+destination alone cannot express.
- **Front-running / claim-jumping:** binding the claim to a buyer-chosen commitment can make a public
  claim specific to the claimant, harder to substitute.

### The open sub-question: what CARRIES the identifier
Do NOT assume — verify-from-source next session:
- [ ] Option 1: reuse the per-UTXO **covenant_id** as the natural nonce. Our covenant model already
      gives every UTXO a unique covenant_id; the offer's covenant_id may already be a unique tag the
      claim can be required to reference, with NO new field invented. CHECK THIS FIRST — likely the
      cleanest.
- [ ] Option 2: an explicit data output (OpReturn-style) on the payment carrying the offer nonce,
      which the covenant reads and requires to match.
- [ ] Option 3 (ChangeNow-inspired, trustless): the offer's own **derived P2SH address** serves as
      the nonce. ANALOGY: when ChangeNow swaps BTC->KAS it generates a unique BTC deposit address
      derived from the swap details — that address IS a built-in nonce (it matches your deposit to
      your specific order). BUT ChangeNow's address is CUSTODIAL (they hold the keys; they take your
      BTC then send KAS from reserves — a trusted, non-atomic intermediary). We want Job 1 (the
      identifier) WITHOUT Job 2 (the custody). Our covenant model ALREADY derives a unique P2SH from
      the compiled contract's constructor params (we saw this: each owner pubkey -> different P2SH).
      So bake the SWAP TERMS (price, seller payout address, optional buyer commitment) into the offer
      covenant's ctor -> its derived P2SH address uniquely encodes the deal. That derived address IS
      the nonce: different terms -> different address; a payment/claim against it is bound to exactly
      these terms by covenant logic; NO custodian. More native than Option 1 because the identifier
      is visible BEFORE the offer is even funded. Most promising for Pattern B.
      CAVEAT: reasoning by analogy from how the TOKEN covenant derives addresses, not from a built
      swap. Verify-from-source: (a) can swap terms be compiled into the offer ctor/address cleanly,
      and (b) can the spending covenant read those terms back for enforcement?
- [ ] Either way: confirm whether Silverscript can read the carrier (covenant_id of a referenced
      input/output, a data-output payload, or ctor-derived terms) as a spend condition.

### Related to BOTH patterns: output-matching ambiguity
When the covenant checks "an output pays X to seller," a multi-output tx needs to know WHICH output
is the payment — a buyer could try to make one output do double duty (payment + change). Robust
defense: bind the payment to a SPECIFIC output index the covenant checks, OR require the payment
output to carry the offer identifier so it cannot be confused with change/unrelated outputs. The
nonce idea is the seed of this defense too. Add a rejection test: a tx where the "payment" output is
actually reused/ambiguous MUST fail.

---

## THE OPEN QUESTION to resolve FIRST (verify-from-source, do not design around a guess)

Can Silverscript express "an output paying >= X to address Y EXISTS in this tx" as a spend
condition? The BitPhoque contract already does output-STATE validation (`validateOutputState`),
and the ecosystem roadmap explicitly anticipates "vault and payment-channel examples" — both
strongly suggest output inspection (value + scriptPubKey of a specific output) is expressible.
But CONFIRM by reading the Silverscript builtins / examples before designing the .sil:
- [ ] Find the builtin(s) that read an output's value and scriptPubKey (e.g. tx.outputs[i].value,
      tx.outputs[i].scriptPubKey) — mirror how the contract reads inputs in checkSigs/witnesses.
- [ ] Check the kcc20 / silverscript example contracts + tests for any payment-enforcing or
      vault/escrow pattern already written — adopt verbatim if it exists (faithful-adopt, no fork),
      exactly like kcc20.sil was adopted for the token itself.
- [ ] Confirm whether the swap condition lives IN the BitPhoque contract (a new entrypoint) or
      as a SEPARATE wrapper covenant the seller's UTXO is moved under. (Likely a new entrypoint /
      variant; weigh against keeping the deployed token contract untouched.)

---

## REJECTION TESTS (the security — non-negotiable, the only real proof)

A swap covenant is trustworthy ONLY if it REJECTS every cheat. Mirror the transfer's
reject-test harness (bitphoque_reject_tests.rs) — self-contained, local VM, accept-AND-reject.
Required cases:

- CONTROL (must PASS): correct swap — phoques to buyer, >= PRICE KAS to seller. Accepted.
- REJECT (must FAIL): buyer takes phoques but pays LESS than PRICE. Underpayment.
- REJECT (must FAIL): buyer takes phoques but pays PRICE to the WRONG address (not seller).
- REJECT (must FAIL): buyer takes phoques with NO payment output at all.
- REJECT (must FAIL): payment present but phoques go to the WRONG owner (not buyer) — should be
  caught by existing ownership/state validation, but prove it in the swap context.
- REJECT (must FAIL): the "payment" output is ambiguous/reused (one output claimed as both payment
  and change) — output-matching attack. The covenant must bind to a specific output / identifier.
- (Pattern B, must FAIL): a coincidental/unrelated payment to seller (no offer identifier) is used
  to claim the offer. With the nonce/covenant_id binding, this MUST be rejected.
- (Pattern B, must FAIL): replay — one payment used to claim a SECOND identical offer.
- MUST HOLD: conservation — no phoques created or destroyed across the swap.
- (Pattern B only) REJECT: claiming an EXPIRED offer; double-claim / front-run a filled offer.

If any "must FAIL" case PASSES, the swap is hollow (a cheat path exists) — DO NOT trust it,
exactly like the no-op-stub trap (checkDataSig) we ruled out for the transfer.

---

## Consensus / mass notes carried from the transfer (will apply to the swap tx too)

The swap tx spends the BitPhoque covenant UTXO, so it inherits everything the transfer learned:
- v1/Toccata input mass = ComputeBudget, NOT SigopCount.
- Budget sized via ComputeBudget::checked_covering_script_units(...) — the swap script may cost
  MORE units than the plain transfer (extra output-checking logic), so re-measure; do not assume
  111,261 carries over.
- Fee floor will be HIGHER again — the swap tx reveals the (possibly larger) redeem script AND
  carries an extra payment output. Re-measure the node's required fee; budget margin.
- Build unsigned -> LOCAL VM VERIFY (covenants_enabled) -> sign -> dry-run -> --go. Never skip
  the local VM verify before broadcast.

---

## Mainnet caveats specific to selling (from bitphoque_mainnet_checklist.md + ecosystem)

- First-sale / first-liquidity is a manipulation surface for thin-float tokens (ecosystem warns
  of unauthorized liquidity pools manipulating price). A swap covenant YOU control is cleaner
  than hoping a fair market forms — you set PRICE and SELLER_ADDRESS in the covenant.
- Owner key = air-gapped (KasSigner), detached signing. The swap harness builds the unsigned tx
  and gets a signature back; it NEVER sees the key. Same rule as the transfer's mainnet form.
- Compile against silverscript V1, not tn10-toc3, before any mainnet swap.
- DEX listing is the liquidity-at-scale path but does not exist yet for covenant tokens
  (months out, post-fork). The swap covenant is the available trustless path NOW.

---

## First concrete step next session
Resolve THE OPEN QUESTION above: read the Silverscript builtins + example contracts to confirm
output value/scriptPubKey inspection is expressible, and check for an existing vault/escrow
example to adopt. THEN draft `bitphoque_swap.sil` (Pattern A) and its reject-test harness.

---

## FUTURE FEATURE: AUCTION (separate, larger build — NOT part of the fixed-price sale)

_Recorded so the design thinking isn't lost. An auction is fundamentally harder than a fixed-price
sale and almost certainly wants its OWN architecture (separate entrypoint(s) or a separate covenant),
NOT a modification of the fixed-price listing. Build the simple provable fixed-price sale FIRST; it
teaches the primitives an auction also needs (output inspection, payment binding, state flags). Do
NOT let auction requirements over-engineer the fixed-price path — they coexist, they don't merge._

### Why an auction is much harder (fixed-price is stateless+atomic; auction is stateful+temporal)
- **Accumulating bid state:** the covenant must track "current highest bid" across MANY transactions
  over time — persistent, evolving state, not a one-shot check. Likely a UTXO whose state holds the
  current high bid + bidder.
- **Bid escrow/commitment:** a bid is only meaningful if the bidder can't walk away — bid funds must
  be locked while the auction runs, then paid to seller (winner) or refunded (losers). Lots of
  custody-of-funds logic, trustlessly.
- **Time boundary:** "auction ends" needs a notion of block-height/locktime to close bidding and
  trigger settlement. (mecenas_locktime.sil suggests locktime IS expressible — promising but unverified.)
- **Refunds for losing bids:** every non-winner must get their committed funds back trustlessly.

### The auction RATCHET (core open question)
Can a Silverscript covenant express "accept this bid ONLY IF it is strictly higher than the current
stored highest bid, and update the stored highest bid"? This compare-and-update-against-stored-state
is the heart of an auction and is UNVERIFIED. Verify-from-source before any auction design.

### Likely architecture (when we get there)
- Probably Option 2 (separate entrypoints) or a SEPARATE covenant — not extend-the-transfer.
- State model: an auction UTXO holding {tokens, highBid, highBidder, endHeight}.
- Entrypoints roughly: openAuction (owner-signed), placeBid (escrow funds, ratchet high bid, refund
  prior high bidder), settle (after endHeight: tokens->winner, highBid->seller).
- Each is its own airtight + reject-tested unit (esp. the unsigned/escrow paths).

### Open questions to resolve FIRST (verify-from-source, do not design around guesses)
- [ ] Bid ratchet: can the covenant compare a new bid to stored highBid and require strictly-greater?
- [ ] Locktime/block-height: confirm the primitive + how covenants read it (start from mecenas_locktime.sil).
- [ ] Bid escrow + refund: can losing-bid funds be returned trustlessly in the same/next tx? Is there
      a vault/payment-channel example to adopt (the ecosystem roadmap mentioned these)?
- [ ] Does the toolchain support MULTIPLE #[covenant] entrypoints cleanly? (We've only used `transfer`.)

### Sequencing
Fixed-price sale (current work) FIRST -> proven + on-chain. THEN auction as a deliberate separate
design+build pass with its own verify-from-source. The fixed-price primitives carry over; the auction
does not constrain the fixed-price design.
## BUILD PROGRESS (updated — sale v2, listing fixed)
The FOR-SALE-STATE model is BUILT and PROVEN at the VM level for the SINGLE-input/output case,
BOTH paths (claim + listing):
- Q1 RESOLVED (compile probe): covenant-bound (binding=cov) entrypoints CAN read tx.outputs. Compose confirmed.
- Q2 RESOLVED (compile probe): forSale(bool)+price(int) state fields compile. State layout {start:1, len:57} (was 46).
- CONTRACT v2: covenants/bitphoque_sale_v2.sil + bitphoque_sale_ctor.json (git c20c2b9). checkAuth branches
  per-input on prevStates[i].forSale: not-for-sale needs owner sig; for-sale needs tx.outputs[witnesses[i]]
  to pay at least price to the seller's P2PK lock. checkSaleState (the v1->v2 FIX) allows for-sale OUTPUTS
  only when NO input is for-sale (a listing, owner-signed); forbids them when ANY input is for-sale (a claim,
  anti-double-claim). v1's checkSaleReset was over-strict: it forbade ALL for-sale outputs, making listing
  IMPOSSIBLE. Caught by reasoning before testing; v2 fixes it.
- SALE+LISTING GATE PROVEN (rust-examples/bitphoque_sale_reject_tests.rs, git c20c2b9):
  CLAIM path: C-CONTROL passes; 6 cheats rejected (underpay / wrong-payee / no-payment / claim-unlisted /
  stays-for-sale / output-match-abuse). LISTING path: L-CONTROL passes (listing WORKS, v1 could not);
  L1 no-sig and L2 wrong-sig both rejected (listing REQUIRES the owner signature).
- DECIDED: per-UTXO sale state, fungible token (NOT per-phoque NFT). checkSaleState = Option 3 reframed
  (forbid for-sale outputs if any input is for-sale).

STILL OPEN (next): generalize beyond SINGLE-input/output to 2x2 FANOUT (checkSaleState "any input for-sale"
  logic should generalize but is untested at fanout; plus multi-claim output-matching: coincidental extra
  payment, cross-referenced witness indices); then ON-CHAIN (re-encounters the v1-mass/ComputeBudget/fee
  rules from the transfer; needs a broadcast + local-VM-verify harness like the transfer; needs a FRESH
  GENESIS since v2's layout/script differ from the deployed token). AUCTION is a separate future build (see end).

---

## PARKED DESIGN NOTES (2026-06-24, decide/address later)

_Captured from a design-notes session while TN10 was mid-IBD. Pointer stub lives in
`bitphoque_status.md`; full reasoning is here._

**#1 — Wallet "gas tank" fee sizing (fees only, separate from purchase price).**
Per-tx floors, proven on-chain:
- _Buyer side (claim):_ single 2-input claim = 900k sompi (0.009 KAS); heaviest 2×2
  multi-parcel claim = 1.5M sompi. A normal one-parcel buy is the single-claim path.
  3 buys ≈ 0.027 KAS, 5 ≈ 0.045, 10 ≈ 0.090 — **under 0.1 KAS for 10 purchases.**
- _Seller/lister side (partial-list, owner-signed single-input):_ script-reveal floor
  ~659k, use 700k sompi (0.007 KAS) per listing.
- _Sizing default:_ seed gas tank with **~0.1–0.2 KAS fee headroom** beyond purchase
  funds (~10–20 claims of runway). Keeps the browser-key risk surface small (gas-tank-
  not-vault). Wrinkles to handle: claims must be funded with spendable KAS *UTXOs* (not
  just total balance); fragmented change can force an extra input → heavier tx; want a
  consolidate primitive eventually.

**#2 — Wallet should send pups (sub-unit), not just whole phoques.**
- _Display:_ 1 phoque = 10^8 pups, same integer under the hood. Offer both views —
  `0.80000000 phoques` (default) and `80,000,000 pups` (power-user toggle). Accept input
  in either; parse the decimal string → BigInt pup count at the input boundary, NEVER
  float math. The list function currently shows whole phoques only — make the formatter
  pup-aware (mainnet-forward; on TN10 the remainder is always 0, the live token has no
  sub-unit headroom).
- _Dust — the real constraint (OPEN, needs source-read):_ two SEPARATE floors.
  (a) the covenant's own dust-floor guard on the partial-list value split — already
  implemented. (b) **Kaspa consensus storage mass (KIP-9)** — the actual trap. Every
  token parcel is a UTXO carrying KAS; the floor is **per-UTXO KAS value, NOT pup count.**
  A 1-pup parcel and a 1M-pup parcel carry identical KAS overhead, so micro-sends are
  mechanically valid but economically silly — each costs the same KAS to clear the
  storage-mass floor. **TODO: derive the real KAS-per-parcel dust minimum from
  `calc_storage_mass` (consensus/core/src/mass/mod.rs, KIP-9 ~line 437) BEFORE the wallet
  hard-codes a "minimum KAS to attach to any token send" constant. Don't infer — read the
  source (enforces → verify-by-reading rule).**

**#3 — Marketing to a specific audience (parked, address pre-launch).**
Three candidate angles, not mutually exclusive — a launch can lead with one and back-fill
the others:
- _Kaspa-native:_ existing KAS holders, non-custodial-comfortable; get fixed-supply /
  no-coordinator / fair-launch without translation. Warmest lead, smallest pond.
- _Meme / culture-first:_ 42,069, the phoque/seal joke, the aesthetic — covenant rigor as
  the substance UNDER the meme, not the pitch.
- _Self-erecting / no-coordinator thesis:_ overlaps the *Revolution* / Oracle-Protocol
  values audience — no minter, no admin, no indexer-as-authority. Values-first.

**#4 — "Boques" = colloquial nickname for phoques (like "bucks" for dollars).**
Casual unit-of-account term — "that'll run you 50 boques." Branding/flavor only, no
contract or wallet impact. Completes the denomination story: formal name (phoque) + base
unit (pup / BitPup) + colloquial (boque), mirroring dollar / cent / buck. Fits the
meme/culture-first angle (#3).

**#5 — Price-escalating emission schedule ("first 100 cost x, next 100 cost xx, …").**
The pattern has a name: a **step bonding curve** (price = deterministic function of
supply-sold-so-far; well-established — Bancor/Uniswap-lineage). NOT exotic. BUT it's an
EMISSION/MINTING mechanism, and that collides with BitPhoque's core thesis (fixed supply
at genesis, NO minter, `isMinter=false`, conservation enforced). A curve needs something
that issues tokens at a rising price = a coordinator. KEY DISTINCTION: trustless ≠
no-coordinator — a curve covenant that can't cheat is still a privileged supply-holder.
This is a DESIGN FORK, decide the MODEL before any curve math:
- **(A) Primary distribution curve** — protocol mints/releases at escalating price. Needs
  a minter covenant holding genesis supply, releasing a parcel only when paid the curve
  price. Contradicts no-coordinator (it's a rule-bound coordinator). Buys a funding ramp +
  "get in early" narrative; costs the no-coordinator purity that is currently BitPhoque's
  whole differentiator. = a DIFFERENT token model, not an add-on.
- **(B) Secondary-market price discovery** — NO new mechanism. v3 sale already does this:
  each holder `partial-list`s their parcel at a self-chosen price. Early-cheap/later-dear
  listings produce an escalating curve EMERGENTLY, market-set not coordinator-set. Fully
  self-erecting. Cost: protocol doesn't CONTROL the curve and doesn't capture the upside —
  early holders do.
- **(Hybrid) Launch-only minter** — a time-boxed genesis auction / tiered initial sale
  distributes the fixed supply at escalating tiers, THEN the minter covenant provably
  self-destructs (supply fully out, conservation takes over forever). Gets the
  distribution-ramp feel WITHOUT a permanent coordinator — the coordinator exists only
  during launch and is demonstrably gone after. The defensible middle: a launch-only
  minter that burns itself is arguably OK; a permanent one isn't. (Apply the baseline's
  "note how mainnet-grade differs from the testnet shortcut" instinct here.)
DECISION DEFERRED — neutral. Pick the model (A / B / Hybrid) before specifying any curve.

**#6 — Wallet clarity: asset/unit disambiguation + new-user on-ramp.**
Three related items:
- **(a) Receive page — KAS vs phoques must be explicit.** Two STRUCTURALLY different
  receive flows share the page today: (i) a **phoque parcel** arrives via signed
  descriptor (`receive` decodes base58check → confirms amount-derived P2SH on-chain — NOT
  a normal address); (ii) **plain KAS** arrives at the derived `kaspatest:`/`kaspa:`
  address (the `m/44'/111111'/0'/0/0` key, gas-tank funding). The page must make the user
  PICK what they're receiving first, then show the right artifact — KAS address + QR for
  funding, or the descriptor-paste flow for an incoming parcel. Concrete wallet-build item,
  not just a label.
- **(b) General confusion audit (pre-publish).** Domain has 5 easily-conflated value
  words: KAS (native gas), sompi (KAS base unit), phoque (token), pup/BitPup (phoque base
  unit), boque (colloquial). Audit EVERY wallet surface (HOME balance, send, receive, list,
  claim, More drawer) — each displayed number must be labeled with its asset+unit; no
  screen silently mixes them. A checklist pass near publish, not a feature.
- **(c) "How to acquire KAS" tutorial for crypto newcomers.** Content/docs work, different
  kind than the wallet code. **MUST verify exchanges / fiat on-ramps LIVE at write-time
  (web search) — which exchanges list KAS, regional availability, fees, who still operates
  all change constantly; do NOT write from memory.** Self-erecting tension to honor: a
  "go to centralized exchange X, do KYC" tutorial sits awkwardly against no-coordinator
  ethos — not a blocker (users need KAS somehow), but the doc should ALSO surface the more
  decentralized paths (mining — Kyle already runs K1Pool — atomic swaps, DEX bridges)
  alongside the easy centralized ones, so the on-ramp doesn't quietly undercut the values
  pitch.
