# BitPhoque — Status

_Read `operating_baseline.md` (HOW) alongside this (WHERE). Stable reference — ABI, consensus rules, contracts, ground facts, file tree — lives in `bitphoque_reference.md` (lookup only). On-chain tx ledger + session narrative in `bitphoque_history.md`. KCC20 redesign intent in `kcc20-findings.md`. BitPhoque lives in `C:\bitphoque` with its own git repo, separate from oracle-protocol and gen5._

---

## CANONICAL PATHS

- Project root: `C:\bitphoque` (own git repo)
- Docs: `C:\bitphoque\docs\`
- Rust harnesses canonical: `C:\bitphoque\rust-examples\`
- Status doc: `C:\bitphoque\docs\bitphoque_status.md` (this file)
- Baseline: `C:\bitphoque\docs\operating_baseline.md`
- Reference: `C:\bitphoque\docs\bitphoque_reference.md` (STALE — Jul 31, predates upstream-watch + pin-assert discipline; see open items)
- KCC20 findings: `C:\bitphoque\docs\kcc20-findings.md`
- SilverScript compiler clone: NOT YET on target pin — see NOW
- Retired TN12 tree: `C:\_RETIRED_kaspa-tn12` (delete after CLI migration)

_OPEN routine step 1: if any path on screen this session is not in this block, STOP and reconcile._

---

## SESSION-START CHECKLIST (any agent, any platform)

1. Read operating_baseline.md (HOW), then this doc (WHERE). Reference + history + kcc20-findings are lookup only.
2. Read the CANONICAL PATHS block above. Any path on screen not in it → STOP.
3. `cd /d C:\bitphoque && git rev-parse --short HEAD && git status` → note head, confirm tree clean, confirm head matches prev-head of last LEDGER sig.
4. Upstream check: kaspanet/silverscript releases/tags, and kaspanet/kccs convention table. Record delta vs last recorded baseline in the opening assessment. Last recorded: v1.0.0 tagged (3ed9733) 2026-08-30-ish; kccs table EMPTY (zero accepted conventions, 1 open PR) as of 2026-09-10.
5. Run the two DEPLOY-TRIGGER CHECKS from the baseline (testnet re-deploy gate, mainnet launch gate). Test each trigger; do not inherit last session's classification.
6. Re-derive blocker classification (absolute vs chosen) per baseline reasoning-discipline.
7. Deliver the six-part opening assessment before any work.

---

<<<NOW>>>

**State (2026-09-10):** BitPhoque feature-complete on TN10 at covenant_id `3baed5a3...` (v4 contract, 4 state fields, 49 bytes). Supply confirmed 42,069 phoques across buyer1 (25,301) + buyer2 (1,001) + buyer3 (15,767). NOT YET redeployed to KCC20 shape. NOT YET rebuilt against SilverScript v1.0.0.

**Upstream resolved this session:** SilverScript **v1.0.0 tagged** (`3ed9733`), final shipped (v1-rc1 `c7d17a1` preceded it). The V1 release that gated all mainnet work since sig #012 is CLEARED upstream. KCC20 is now a **book in the silverscript repo** (canonical, alongside TUTORIAL.md + DECL.md), no longer a forum spec. A canonical v1 covenant descriptor exists (commits to CAIP-2 network, program SHA-256, covenant id, ABI, state layout, control-principal + operation-auth principals). Mainnet KCC20 indexers are LIVE (kcc20.info, kaslab/HUB21) — the composability payoff from kcc20-findings is now real, not hypothetical. Argent language shipped (compiles .ag → .sil, has worked kcc20_asset + minter examples, open/closed ICC, virtual-slot state) — may already generate the multi-in/multi-out array-transfer form that kcc20-findings flagged as the one unexercised authoring construct.

**Still GUESSED (the load-bearing unknown):** whether BitPhoque's layout still aligns with KCC20 between pin `4b0e1cd` (the pin kcc20-findings reviewed) and tag `v1.0.0`. The docs.kaspa.org ownership branch matches the recorded shape (3-way identifierType split), but that is a docs page, not a byte-diff of the book + examples at the tag. This is the TESTNET RE-DEPLOY trigger and it has NOT been run.

**kccs formal status:** table EMPTY. KCC20 is still a DRAFT convention. Per baseline mainnet trigger, ratification is a NOTE not a hard gate — ecosystem runs mainnet indexers against the de-facto book shape ahead of ratification.

**Single next action:** Run the shape-stability diff (TESTNET RE-DEPLOY trigger). Fetch silverscript at `4b0e1cd` and `v1.0.0`, diff the KCC20 book + kcc20.sil + kcc20-minter.sil, read for surface-movers on the four-field layout / three transfer gates / controller anti-delegation requires. ZERO diff on that surface converts the chosen blocker to "safe to build"; any diff is the breaking-change signal.

<<<END NOW>>>

---

## DESIGN — SETTLED

- **Architecture:** per-UTXO balance, conservation-enforced.
- **Ownership:** per-UTXO signature, `IDENTIFIER_PUBKEY=0x00` (plain checkSig).
- **Supply:** FIXED at genesis, NO minter. 42,069 phoques. `genesisIsMinter=false`.
- **Subdivision:** 1 phoque = 100,000,000 BitPups (8 dp). TN10 minted whole; mainnet needs re-genesis for sub-unit headroom.
- **Sale model (v3):** per-UTXO `forSale` + `price`. Listing = owner-signed. Claim = payment-authorized.
- **Wallet custody:** non-custodial, BIP-39 12-word, key in-browser, never on server. Encryption mandatory (no session-only bypass).
- **Receive:** registry-based discovery (primary) + offline descriptor codec (fallback, 118B base58check).
- **KAS handling:** "gas-tank, not vault." Fee floors: 700k list/split, 900k claim, 1.3M merge.

### KCC20 redesign direction (SETTLED intent, not yet built — full detail in kcc20-findings.md)

- BitPhoque becomes the **KCC20 asset covenant** (four-field state: ownerIdentifier / identifierType / amount / isMinter; three transfer gates: ownership-auth, supply conservation, minter-taint).
- A **gen5 reputation/checkpoint covenant becomes the controller** (owns the asset's minter branch by covenant-id). Reputation-gated issuance enforced by co-spend. Every mechanical piece already gen5-proven on TN10.
- Start on the singleton controller/mint path (within proven patterns). The multi-in/multi-out array-transfer form is the one genuinely new authoring construct — check whether Argent generates it before hand-authoring.
- Adopted KCC20 guards become BitPhoque's guards to audit at the target pin. "It's the standard" ≠ "it's audited."

<<<OPEN ITEMS>>>

### Blockers (reclassified 2026-09-10)
- [ ] **TESTNET RE-DEPLOY trigger** — run the shape-stability diff (4b0e1cd vs v1.0.0). CHOSEN blocker (waiting on our own verification), was absolute pre-v1.0.0. This is the live next action.
- [ ] **CLI binary/network mismatch** — TN12-era CLI (`C:\_RETIRED_kaspa-tn12`, v1.2.1-toc.3) derives wrong P2SH. Must rebuild against v1.0.0 and verify by actual import. More urgent now: v1.0.0 compiler derives different addresses again.
- [ ] `getUtxosByCovenantId` RPC — feature request drafted, not filed on kaspanet/rusty-kaspa. Would eliminate parcel registry as hard dependency.

### Mainnet path
- [ ] Satisfy MAINNET LAUNCH trigger (baseline) — all five conditions.
- [ ] v4 downstream code sync (sigscript_v4 done; descriptor OK; compiled_real/CLI/wallet were blocked on V1 compile — now unblocked, pending KCC20 conformance decision).
- [ ] Re-genesis produces new `covenant_id` — sweep `3baed5a3...` in all JS + harnesses.
- [ ] CLI .key files have no BIP-39 backup — mainnet genesis should mint to BIP-39 wallet key.
- [ ] Clear `bitphoque_mainnet_checklist.md`.
- [ ] Evaluate launch-proof convention (covenant-id-preimage proof, full contracts + initial state, source-auditable) — Sutton flagged as emerging norm; confirm whether it entered the standard.

### Cleanup / minor
- [ ] **reference.md is STALE** (Jul 31) — predates upstream-watch + pin-assert discipline the baseline now requires; no canonical-paths / pin fields. Sharpen pass needed.
- [ ] Migrate status/reference/history to full marker-zone + six-doc structure (this rebuild started it for status; reference + deployments.md still owed; no deployments.md exists yet).
- [ ] cPanel: evaluate whether still needed (outlawpolitics.org redirect).
- [ ] D: build-clone bin sweep (recopy 14 harnesses → clone, rebuild).
- [ ] Delete `C:\_RETIRED_kaspa-tn12` (after CLI migration).
- [ ] Buy transactions don't record seller as counterparty in tx history.
- [ ] index.html 'get started' section — small copy updates (receive/send done; rest reference old flows).

### Parked
- Auction — separate future build (stateful/temporal). See `bitphoque_swap_covenant_design.md`.
- SMS/Twilio node monitor.
- Price-escalating emission (step bonding curve).
- Key naming — legacy buyer1/2/3 rename sweep.

<<<END OPEN ITEMS>>>

---

<<<LEDGER>>>

_Append-only. Newest first. Prior full sigs (≤ #012) in `bitphoque_history.md`._

---sig #012 | 2026-07-11 ~EST | desktop | trees: bitphoque | scope: descriptor UX overhaul, registry-based discovery, tx history receives, show recovery phrase, supply accounting, explorer/RPC feature request draft | head: 4e32af4

---sig #013 | 2026-07-12 ~EST | desktop | trees: bitphoque | scope: kas-smith forum analysis (KCC20 spec, borrowed receive, open ICC/Capsule/virtual state), forum post shipped, CLI import debugging (TN12/TN10 mismatch identified), operating baseline trimmed, status doc trimmed | head: 6d603bf
DRIFT: CLI import universally failed — only wired Cargo project is TN12-era (v1.2.1-toc.3), likely wrong P2SH for TN10. Pivoted to diagnosis. Forum work productive — shipped post, analyzed KCC20 alignment + borrowed receive + open ICC.

---sig #014 | 2026-09-02 ~EST | desktop | trees: bitphoque | scope: filed kcc20-findings.md (KCC20 redesign intent, adopt-when-settled, gen5 controller join); split status→volatile status + stable reference; receive/send copy; removed session-only wallet bypass | head: 28aa9f9

---sig #015 | 2026-09-10 ~EST | desktop | trees: bitphoque | scope: upstream re-derivation session (no code). Confirmed SilverScript v1.0.0 tagged (3ed9733) — mainnet blocker CLEARED upstream. Confirmed kccs table EMPTY (KCC20 still draft convention). Confirmed mainnet KCC20 indexers live (kcc20.info, kaslab). Noted Argent shipped w/ KCC20 examples. Rebuilt status doc old→marker-zone format + folded findings. Added deploy-trigger checks (testnet re-deploy / mainnet launch) to operating_baseline.
CONFIRMED: v1.0.0 exists (git ls-remote tags); kccs empty (fetched repo, header row only); indexers live (kcc20.info, kaslab.space). At: their canonical sources this session.
GUESSED (flagged, nothing load-bearing left as guess): layout alignment 4b0e1cd↔v1.0.0 holds — inferred from docs.kaspa.org ownership branch, NOT byte-diffed. This is the pending TESTNET RE-DEPLOY trigger; no downstream work proceeds on it until diffed.
DRIFT: opened on the premise "SilverScript V1 is deployed, rework to KCC20." Reality: V1 is UPSTREAM's release, not a BitPhoque deploy — nothing in-tree shows a re-genesis or v1 compile. The Sep 2 settle-test (kccs acceptance) is still UNMET; what changed is de-facto freeze (live mainnet indexers), not ratification. So the trigger is not "start rework" but "run the shape-stability diff first." Corrected the premise before authoring any conformance plan.
prev-head: 28aa9f9

<<<END LEDGER>>>
