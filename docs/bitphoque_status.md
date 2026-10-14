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

**State (2026-10-14):** BitPhoque feature-complete on TN10 at covenant_id `3baed5a3...` (v4, bespoke 4-field shape). Direction set this session: conform to KCC20 as a STANDALONE fixed-supply, pubkey-owned asset — 42,069 phoques, no minter, no controller, no inter-covenant coupling. gen5 join REMOVED from scope. The recorded KCC20 shape (kcc20-findings.md, pin 4b0e1cd) is NON-CONFORMANT — the shape-stability diff FAILED.

**Shape delta (confirmed at kccs main, 2026-10-14):** KCC20State is now SIX fields — `amount`(int), `owner`(byte[32]), `owner_scheme`(byte), `borrow_scheme`(byte), `borrow_guard`(byte[32]), `extension_commitment`(byte[32]). `isMinter` and the minter-taint gate are gone. Owner schemes are KCC2: pubkey owners use `owner_scheme` 0x00 (p2pk-schnorr), where the authority is the Schnorr pubkey itself — maps onto the existing per-UTXO checkSig. Borrowed-receive is mandatory (3 enabled schemes). Default transfer is the array form `transfer(KCC20State[],byte[])`, bounded 3-in/3-out. Transfer preserves total amount across the covenant family (conservation-only — exactly what fixed-supply wants).

**Conformance-target status:** kccs table lists KCC1/KCC2/KCC20 as Last Call; the KCC1 and KCC20 FILES still self-label Status: Draft (KCC2 reads Last Call). Asset-side shape not frozen by the files' own headers — re-run the diff if it moves.

**Open sub-question (sets the state layout):** BitPhoque's sale model (v3/v4 `forSale`+`price`). Covenant-enforced → rides in `extension_commitment` as a KCC1 §10 virtual element (32-byte commitment in state, opened in witness, verified against it). Registry-based like receive discovery → base KCC20 state suffices, no extended state. The v4 contract settles it; unread this session.

**Single next action:** read the v4 contract's sale-model enforcement (covenant-level vs registry) to fix whether `extension_commitment` is used, then scope the 6-field conform.

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

### KCC20 conform direction (SETTLED 2026-10-14 — standalone fixed-supply, not yet built)

- BitPhoque becomes a STANDALONE **KCC20 asset covenant**: fixed supply (42,069 phoques), no minter, owners are pubkeys (`owner_scheme` 0x00, p2pk-schnorr). No controller, no issuance, no inter-covenant coupling.
- State is the KCC20 six-field layout: `amount`(int) / `owner`(byte[32]) / `owner_scheme`(byte) / `borrow_scheme`(byte) / `borrow_guard`(byte[32]) / `extension_commitment`(byte[32]). Confirmed at kccs main 2026-10-14; the recorded 4-field shape in kcc20-findings.md is superseded.
- Mandatory conformance surface: borrowed-receive (all three enabled schemes) and the default array transfer `transfer(KCC20State[],byte[])`, bounded 3-in/3-out. Supply conservation only — the leader preserves total token amount across the covenant family.
- Sale model (`forSale`/`price`): if covenant-enforced, carried in `extension_commitment` as a KCC1 virtual element; if registry-based, base state suffices. v4 contract settles it.
- Adopted KCC20 guards become BitPhoque's guards to audit at the target pin. "It's the standard" ≠ "it's audited."
- gen5 join REMOVED from scope (2026-10-14): it was an idea that hardened into a design rule. BitPhoque is its own thing. gen5 may later build a separate token with lessons learned from BitPhoque; nothing BitPhoque ships accommodates it.

<<<OPEN ITEMS>>>

### Blockers (reclassified 2026-10-14)
- [x] **TESTNET RE-DEPLOY trigger — RUN 2026-10-14, FAILED.** Diffed recorded shape vs kccs main. Non-conformant: KCC20 now 6-field (amount/owner/owner_scheme/borrow_scheme/borrow_guard/extension_commitment); isMinter + minter-taint gone; covenant-id scheme 0x02→0x04; borrowed-receive mandatory; array transfer default. The single load-bearing GUESS (layout alignment) is retired FALSE. Superseded by the conform item below.
- [ ] **Conform BitPhoque to KCC20 6-field shape** — standalone fixed-supply, pubkey owners (owner_scheme 0x00). Rebuild state to KCC20State; implement mandatory borrowed-receive (3 schemes) + default array transfer (3-in/3-out); supply conservation only. Blocked on: (a) sale-model decision — extension_commitment virtual element vs registry; (b) target compiler-pin selection.
- [ ] **CLI binary/network mismatch** — TN12-era CLI (`C:\_RETIRED_kaspa-tn12`, v1.2.1-toc.3) derives wrong P2SH. Must rebuild against the target pin and verify by actual import. More urgent now: v1.0.0 compiler derives different addresses again.
- [ ] `getUtxosByCovenantId` RPC — feature request drafted, not filed on kaspanet/rusty-kaspa. Would eliminate parcel registry as hard dependency.

### Mainnet path
- [ ] Satisfy MAINNET LAUNCH trigger (baseline) — all five conditions.
- [ ] v4 downstream code sync (sigscript_v4 done; descriptor OK; compiled_real/CLI/wallet were blocked on V1 compile — now unblocked, conform decision made (KCC20) — re-sync to the 6-field shape).
- [ ] Re-genesis produces new `covenant_id` — sweep `3baed5a3...` in all JS + harnesses.
- [ ] CLI .key files have no BIP-39 backup — mainnet genesis should mint to BIP-39 wallet key.
- [ ] Clear `bitphoque_mainnet_checklist.md`.
- [ ] Evaluate launch-proof convention (covenant-id-preimage proof, full contracts + initial state, source-auditable) — Sutton flagged as emerging norm; confirm whether it entered the standard.

### Cleanup / minor
- [ ] **kcc20-findings.md superseded** — recorded shape (4-field, scheme 0x02, isMinter, KCC20Minter controller, gen5 join) is non-conformant and out of scope. Rewrite to the standalone conform intent, or mark superseded and point to NOW.
- [ ] Push: local master 4 commits ahead of origin at 16aa57d. Run the secret-sweep, then push.
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
---sig #016 | 2026-10-14 ~EST | desktop | trees: bitphoque | scope: re-entry after ~1mo gen5 drift. Re-ran upstream check (kccs table now populated: KCC0 Final, KCC1/2/20 Last Call, KCC23 Draft). Ran the shape-stability diff against kccs main — FAILED: KCC20 moved 4-field→6-field, isMinter+minter-taint removed, covenant-id scheme 0x02→0x04, borrowed-receive now mandatory, array transfer now default. Removed gen5 controller join from BitPhoque scope. Chose: conform BitPhoque to KCC20 as standalone fixed-supply pubkey-owned asset. Rewrote DESIGN subsection + NOW; reclassified blockers.
CONFIRMED: KCC20/KCC2/KCC1 field layouts, owner-scheme bytes (covenant-id=0x04, pubkey=0x00), covenant-id proof via OpCovInputCount (KCC2 §2.3.2), leader/delegator model (KCC1 §9.1), extension_commitment = KCC1 §10 virtual element — all read at raw.githubusercontent.com/kaspanet/kccs main this session. Table statuses at github.com/kaspanet/kccs. HEAD 16aa57d, tree clean, 4 ahead of origin.
GUESSED (flagged, nothing load-bearing left in scope): covenant-id builtin roster + any output-side cid-capture builtin live in KIP-20 (unread) — out of scope now that minter/ICC is dropped. Sale-model enforcement (covenant vs registry) unread — gates whether extension_commitment is used.
DRIFT: opened to "prepare for mainnet" expecting the recorded KCC20 shape to be the target — it isn't, the spec moved under the findings doc. Second correction was the user's: I carried the gen5 controller join forward as load-bearing design; it was an idea elevated to a rule and was never part of BitPhoque's fixed-supply economics. Dropping both shrank the critical path to a plain asset conform. PIN-ASSERT skipped (no compiler clone pinned). Git OPEN step run mid-session — no folder bridge this session, HEAD pasted by hand.
prev-head: 16aa57d

<<<END LEDGER>>>
