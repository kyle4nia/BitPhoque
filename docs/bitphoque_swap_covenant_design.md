# BitPhoque — Swap Covenant Design Note (the trustless-sale mechanism)

_Status: DESIGN ONLY — not built. This is the spec to start from next session. It is the
answer to "how do I sell real BitPhoques to a stranger without a middleman." Same stack,
same discipline as the transfer harness (which is DONE + proven on TN10): write it, read
the source for what enforces, local VM verify, REJECTION TESTS for every cheat, TN10 before
mainnet. Read alongside bitphoque_status.md and bitphoque_mainnet_checklist.md._

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

## Two patterns (Pattern A is the first build)

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
- [ ] Either way: confirm whether Silverscript can read the carrier (covenant_id of a referenced
      input/output, or a data-output payload) as a spend condition.

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
