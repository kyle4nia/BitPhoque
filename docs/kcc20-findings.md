# BitPhoque + KCC20 - Findings and Redesign Intent

> **⚠ SUPERSEDED — 2026-10-14 (sig #021). Historical record; do not design from this doc.**
> The current design is in `bitphoque_status.md` NOW + DESIGN. What changed since this was filed:
> - KCC20 moved **4-field → 6-field** (amount / owner / owner_scheme / borrow_scheme / borrow_guard / extension_commitment); `isMinter` + minter-taint are gone.
> - covenant-id owner scheme **0x02 → 0x04**; borrowed-receive now mandatory; array transfer default.
> - The **minter / KCC20Minter controller** model and the **gen5 controller join** are **out of scope** — BitPhoque is now a standalone, fixed-supply, **no-minter** pubkey-owned asset (gen5 join removed sig #016).
> - Toolchain is **Argent** on `argent-lang/kcc20-reference` (not hand-authored `.sil` at silverscript pin 4b0e1cd).
> The "adopt when the spec settles" posture also resolved: the decision is to **conform + exercise on TN10 now, HOLD mainnet genesis** until KCC-20 is Final + ABI-converged (kccs #28).

_Filed 2026-09-02 from a review of the KCC20 convention (Kaspa's covenant-native fungible-token standard) at silverscript pin 4b0e1cd. Source read: docs\kcc20-book (overview + contract + minter chapters), silverscript-lang\tests\examples\kcc20.sil, kcc20-minter.sil. This doc is the BitPhoque-side record; the gen5-side note lives in the gen5 tree._

## The decision

BitPhoque should be redesigned to the KCC20 shape WHEN THE SPEC SETTLES. Not before. The reasoning and the caveats are below.

## Why adopt (the standard-interface argument)

A fungible token that is not KCC20-shaped is invisible to the ecosystem's infrastructure. Every covenant indexer (kascov.io, KaspaCom covenant explorer, kaslab) and every covenant-ready wallet (Kaspire) decodes the KCC20 four-field state layout and nothing else. A bespoke BitPhoque layout means building and maintaining a private explorer + wallet integration indefinitely, for no benefit. Adopting the standard is free composability: BitPhoque tokens show up in directories, wallets, and DEX/market tooling automatically, with a per-token validation verdict.

This is the standard-interface lens: design for composability through a shared interface rather than bespoke coupling. KCC20 is that interface for fungible tokens on Kaspa.

## Why "when it settles" is load-bearing

KCC20 is a DRAFT, not an accepted standard, as of this writing:

- The official `kaspanet/kccs` (Kaspa Calls for Conventions) repo is empty - zero accepted conventions, one open PR.
- The spec lives in Ori Newman's book inside the compiler repo, explicitly labelled draft / proof-of-concept.
- The naming is not fixed (Newman: "Kaspa Contract Convention"; the org repo: "Kaspa Call for a Convention").
- A community workshop building KCC20 tokens (in the Argent language) is scheduled 2026-09-05 - a sign the spec is maturing, not that it is done.

Building against a draft means eating every breaking change the standardization process produces. The right posture: track KCC20 as a NOTE, redesign BitPhoque to it once it is accepted and version-stable. Do not adopt a moving target.

## The KCC20 shape (what BitPhoque would become)

KCC20 splits into TWO covenants with separate responsibilities. This separation is the main architectural point.

### The asset contract (KCC20) - the token state machine

Each token UTXO carries four state fields:

- `ownerIdentifier` (byte[32]) - who owns this UTXO
- `identifierType` (byte) - how to interpret the owner: 0x00 pubkey / 0x01 P2SH script hash / 0x02 covenant id
- `amount` (int) - the token balance in this UTXO
- `isMinter` (bool) - whether this UTXO may create supply

A token is not a global ledger. It is a SET of UTXOs, each with a balance and an owner - like KAS itself, but with covenant-enforced state. Transfer = co-spend token UTXOs, produce new ones.

Three gates on transfer:

1. **Ownership authorization** - a branch on `identifierType`. Pubkey owners sign; P2SH owners prove a matching input scriptPubKey; COVENANT_ID owners are proven by `OpInputCovenantId(witnesses[i]) == ownerIdentifier` - the transaction must actually spend a UTXO owned by that covenant. There is no eval / code-reference; the proof of "owned by covenant C" IS the co-spend of a C-owned input.
2. **Supply conservation** - for non-minters, sum(input amounts) == sum(output amounts). Tokens are neither created nor destroyed in a transfer.
3. **Minter-taint** - a non-minter cannot produce minter outputs. Mint authority only descends from a minter input; it can never be forged or bootstrapped by a holder.

### The controller covenant (KCC20Minter) - the issuance policy

Separate covenant. Owns a privileged minter branch of ONE token instance (by covenant-id ownership) and meters issuance. Its state: the controlled token's cid, a remaining-issuance allowance (`amount`), an `initialized` latch, plus template metadata (prefix/suffix len + expected template hash) to authenticate the token bytes it co-spends.

- Mint accounting: minted = sum(token outputs) - token input; the controller debits that from its own allowance; `require(newState.amount >= 0)` stops minting at budget exhaustion. Bounded supply from an unbounded-in-time minter.
- Anti-delegation guards (the security keystones): the minter output must stay owned by the controller's own cid (`OpInputCovenantId(this.activeInputIndex)`), must stay a minter, and the recipient output must NOT be a minter. Authority cannot leak or transfer.

### Terminology (adopt this vocabulary for BitPhoque)

- **asset covenant** (A) - the token state machine. Reusable; defines what a valid token transition is.
- **controller covenant** (C) - the external policy covenant whose cid owns the asset's minter branch. Defines under what policy new supply may be issued.
- **issuance** = the policy governing when supply may be created. **minting** = the concrete tx-level act of creating it.
- The minter branch is NOT owned by an admin pubkey directly - it is owned by covenant id C. The admin key authorizes the controller SCRIPT; the script decides how its authority is used.

### Lifecycle (three phases)

1. **minter genesis** - create controller C (normal covenant genesis), `initialized=false`, placeholder token cid.
2. **asset genesis** - in ONE tx, create token A (a minter branch owned by C, amount 0) AND spend C through `init`, which captures A's cid via `OpOutputCovenantId(0)` and latches `initialized=true`. (This is the cross-covenant cid-capture: C cannot bake A's cid at C's own genesis because A does not exist yet; init fills it in once, in the tx that mints A.)
3. **issuance** - co-spend the token minter branch + the controller together; each verifies its own side; controller decrements its allowance.

Each covenant verifies only the part of the transaction it owns. They do not collapse into one giant-policy script.

## The strategic point: the gen5 controller join

This is why KCC20 is more than "a standard to adopt."

KCC20's controller slot accepts `identifierType == COVENANT_ID` - the minter branch can be owned by ANY covenant. That controller can be a **gen5 reputation or checkpoint covenant**. Consequences:

- BitPhoque issuance can be gated by gen5 reputation - the controller is not a bare decreasing-budget, it is the gen5 reputation substrate deciding WHEN supply may be created.
- Newman's own rate-limit example (mint only if `time > nextIssuance`, then `nextIssuance += 30 days`) is STRUCTURALLY identical to gen5's checkpoint cadence gate (a covenant storing a monotone clock, gating an action, advancing the clock) - which gen5 has built and fired on-chain (checkpoint #074).
- Every mechanical piece of the join is already gen5-proven on TN10: the OpInputCovenantId ownership check, the co-spend-is-the-proof ICC, the 2-input co-spend shape, the paired readInputState/validateOutputState template machinery.

So the designed convergence is: **BitPhoque = the KCC20 asset; a gen5 rep/checkpoint covenant = the controller.** Reputation-gated token issuance, enforced by co-spend, using the same builtins gen5 already fired. KCC20's controller/asset split is the interface that makes the join clean instead of a bespoke bolt-on.

## Caveats to carry into the redesign

1. **Settled-form only.** Do not build against the draft. Re-check spec status (kaspanet/kccs acceptance, version stability) before starting.
2. **The adopted guards become BitPhoque's guards to audit.** KCC20's security rests on the minter-taint rule, the anti-delegation requires, and supply conservation. Adopting the shape does not make them safe by default - they must be read and verified at whatever compiler pin BitPhoque targets (verify-by-reading-the-source for anything that enforces). "It's the standard" is not "it's audited."
3. **The multi-in/multi-out array form is unexercised.** The general KCC20 transfer uses `#[covenant(binding=cov, from=N, to=M)]` with State[] arrays (N tokens -> M tokens atomically). This is a covenant-authoring form not yet exercised in the gen5 tree (all gen5 covenants are singletons). The controller/mint path is singleton-constrained and IS within proven patterns; start there. The array transfer is the one genuinely new authoring construct a KCC20 BitPhoque needs.

## Not doing now

This is forward intent, filed while BitPhoque is not the active tree. No BitPhoque redesign work starts here. The trigger to act: KCC20 reaches accepted/stable status AND BitPhoque becomes the active project. Until then: track KCC20 as a NOTE, keep the controller-join design in mind so nothing gen5 builds forecloses it.
