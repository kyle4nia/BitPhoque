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

**State (2026-10-14):** DESIGN COMPLETE → build phase. BitPhoque = single-covenant KCC20 asset in Argent on argent-lang/kcc20-reference `kcc20.ag`. Fixed supply 42,069, pubkey-owned (0x00), on-chain sale. Toolchain proven in-environment (C:\kcc20-reference, cargo test --locked 60/60). Full design settled (see DESIGN): six-field state; sale = int price sompi (0 idle/>0 listed) in extension_commitment = BLAKE3(0x01 || LE64(price)); set_price + claim, BOTH singleton, alongside a byte-identical standard transfer; genesis pre-mints full supply, no minter. Not yet: BitPhoque Argent project, entrypoint .ag, genesis, TN10 deploy.

**Mainnet caution (unchanged):** KCC-20 Last Call/Draft-in-file/NOT Final; ABI not converged (kccs #28). Conform + exercise on TN10 now; HOLD genesis until Final + converged; re-diff if spec moves.

**Toolchain pins (record in reference.md):** argent + argent-runtime @ argent-lang/argent 9a9f4b10; rusty-kaspa a41a333b (must match TN10 for deploy); Rust 1.94.1 floor (local 1.96.0).

**Single next action:** scaffold the BitPhoque Argent project — a cargo project pinned to the same argent/rusty-kaspa revs, with contracts/bitphoque.ag starting as the reference kcc20.ag (standard transfer+delegator unchanged), ready for the set_price + claim entries and a rejection-test suite.

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

### KCC20 conform direction (SETTLED 2026-10-14 — single-covenant, Argent, on-chain sale)

- BitPhoque = a STANDALONE single-covenant KCC20 asset, authored in Argent, built on argent-lang/kcc20-reference `contracts/kcc20.ag`. Fixed supply 42,069, pubkey owners (owner_scheme 0x00), no minter.
- State = KCC20 six-field layout (amount/owner/owner_scheme/borrow_scheme/borrow_guard/extension_commitment). The reference supplies the standard transfer + delegator, all owner schemes, all borrow schemes, conservation, extension preservation — kept byte-identical for vector-conformance.
- On-chain sale lives in `extension_commitment` as a KCC1 §10 virtual element holding `{forSale, price}`; `list`/`claim`/`cancel` are BitPhoque-specific entrypoints added alongside the standard transfer. v4's payment-authorized claim (pay-to-owner, no seller sig) ports in. No escrow covenant, no covenant-id ownership. Listed token = own fungibility class.
- Toolchain: ADOPT ARGENT (compile .ag -> .sil/bytecode). New commitment; the tree was previously hand-authored .sil. Argent + its silverscript dep get pinned under the environment-version-trap discipline.
- Adopted KCC20 guards + borrowed-receive become BitPhoque's to audit at the target pin. "It's the standard" != "it's audited."
- gen5 join REMOVED from scope (2026-10-14). BitPhoque is its own thing; gen5 may later build a separate token with lessons learned. Note: the single-covenant sale needs NO inter-covenant machinery, so nothing here reopens the gen5 coupling.

### Extension layout (SETTLED 2026-10-14 — genesis-irreversible)

- Sale state = a single `int price` in KAS sompi. `price == 0` → idle (not listed); `price > 0` → listed. forSale folded into price (price=0,forSale=true was already disallowed in v4).
- `extension_commitment` = BLAKE3 commitment (KCC1 §10 virtual element) over the packed value with a 1-byte version tag:
    Packed(ext) = 0x01 || LE64(price)        (9 bytes: version + 8-byte LE signed int, KCC1 §5.3)
    extension_commitment = BLAKE3(Packed(ext))
  idle = BLAKE3(0x01 || LE64(0)) — one fixed constant; all idle tokens share it and stay mutually fungible.
- Raw price is the opening, supplied as witness to the sale entrypoints; the standard transfer never opens it, only preserves the 32 bytes. Price is hidden on-chain; human-readable price lives in the bitphoque.org registry, trustlessly verifiable as BLAKE3(0x01 || LE64(advertised_P)) == on-chain commitment.
- Version byte reserves the seam for a future extension field without a format collision (solve-for-this-version; genesis ossifies the format).
- Price is per-UTXO, not per-unit (as v4). Merging two listed UTXOs via standard transfer keeps one price on the whole — a wallet concern, not a contract hole.

### Sale entrypoints (added alongside the reference transfer; standard transfer stays byte-identical)

- `set_price(new_price, sig)` — owner-authorized, SINGLETON. 0→P lists, P→P' reprices, P→0 cancels. Owner sig required; owner/amount/borrow preserved; only extension_commitment changes to BLAKE3(0x01 || LE64(P')).
- `claim(buyer_owner, buyer_owner_scheme, opened_price P, pay_idx)` — payment-authorized, NO seller sig, SINGLETON (OpCovInputCount == 1). Binds the opening (BLAKE3(0x01 || LE64(P)) == commitment), requires P > 0 and owner_scheme 0x00, requires tx.outputs[pay_idx] == P2PK(owner) with value >= P, produces a buyer-owned output forced to idle (commitment = BLAKE3(0x01 || LE64(0))), amount conserved.
- Standard `transfer` / `transfer_delegator` unchanged → vector-conformant, indexers decode balances.
- CLAIM ACCOUNTING — RESOLVED 2026-10-14: claim is SINGLETON (one listed BitPhoque input per tx). Batched multi-listing claim (v4's strictly-increasing payment-index / one-payment-per-seller guard under leader/delegator) is DEFERRED — later isolated optimization, clean seam, not a v1 gate. Singleton removes the cross-input double-count and coverage-gap bug class entirely; the standard transfer still does full 3-in/3-out for idle move/merge/split.
- Security vectors (singleton claim, reasoned not yet VM-tested): underpay → fails commitment bind; claim-without-pay → payment require; stays-listed → commitment forced idle; non-owner sweep via transfer → seller sig required; bundling a victim listing → singleton OpCovInputCount==1; partial-fill → amount conserved. All rejected. These become rejection tests in the BitPhoque suite.
- Minor-open: claimed output borrow_scheme — force disabled (0x00) [chosen] vs buyer-chosen. KAS/dust floors are wallet-side.

<<<OPEN ITEMS>>>

### Blockers (reclassified 2026-10-14)
- [x] **TESTNET RE-DEPLOY trigger — RUN 2026-10-14, FAILED.** Diffed recorded shape vs kccs main. Non-conformant: KCC20 now 6-field (amount/owner/owner_scheme/borrow_scheme/borrow_guard/extension_commitment); isMinter + minter-taint gone; covenant-id scheme 0x02→0x04; borrowed-receive mandatory; array transfer default. The single load-bearing GUESS (layout alignment) is retired FALSE. Superseded by the conform item below.
- [ ] **Build BitPhoque as single-covenant KCC20 (Argent).** Base = argent-lang/kcc20-reference kcc20.ag, extended with list/claim/cancel over an extension_commitment virtual element carrying {forSale, price}. Sub-steps: (a) stand up + pin the Argent toolchain, compile the reference unchanged; (b) design idle vs listed commitment layout; (c) author list/claim/cancel (port v4 payment-claim); (d) exercise on TN10 against the target pin.
- [ ] **Argent toolchain setup + pin** — clone/build Argent + its silverscript dependency; record the pin in reference.md (environment-version-trap: binary must match TN10). PIN-ASSERT gets an Argent-pin check once set.
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
- [ ] **Verify indexer acceptance of a no-minter pre-minted KCC20 family** (kcc20.info / kaslab) — the reference assumes PublicMint/TokenSeed; confirm a direct-genesis fixed-supply token decodes. Composability gate.
- [ ] **Hold mainnet genesis until KCC-20 is Final + ABI converged** (kccs #28). Re-run the shape diff vs the ratified spec before genesis.
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
---sig #017 | 2026-10-14 ~EST | desktop | trees: bitphoque | scope: conform-path design arc (no code). Settled: sale is on-chain (covenant-enforced, confirmed by reading v4); KCC20 6-field re-verified as current; sale architecture = single covenant, {forSale,price} in extension_commitment virtual element + list/claim/cancel entrypoints, standard transfer kept byte-identical; toolchain = ADOPT ARGENT on argent-lang/kcc20-reference; genesis = pre-mint fixed supply, no minter.
CONFIRMED (at raw.githubusercontent kccs main + argent-lang/kcc20-reference master + GH Pages kcc20-book, this session): KCC1/2/20 layouts; owner_scheme covenant-id=0x04; OpCovInputCount proof; leader/delegator; extension_commitment is required identical-across-family and preserved in the reference transfer code; reference kcc20.ag = 6-field with 5 owner + 4 borrow schemes, michaelsutton finalize Oct 7; silverscript v1.0.0 example is still old 4-field (kccs issue #28); v4 sale is covenant-enforced pay-to-owner.
GUESSED (flagged): indexers accept a no-minter pre-minted KCC20 family (reference assumes PublicMint/TokenSeed) — unverified. KCC-20 shape stable through Final — it is Last Call/Draft-in-file, ABI not converged (kccs #28); may shift, re-diff before genesis.
DRIFT: opened the arc expecting a small conform; two corrections landed. (1) Reading v4 showed the sale is covenant-enforced, and the reference transfer locks extension_commitment, so the sale had to become separate entrypoints over a virtual element, not a field in transfer. (2) My first reference read (silverscript example) was stale 4-field and I wrongly concluded no 6-field reference existed; user pushed on staleness — the real 6-field reference is argent-lang/kcc20-reference. Net: hard parts (transfer, borrowed-receive) are proven by a core author; BitPhoque's only novel work is the sale entrypoints.
prev-head: 5b5071b
---sig #018 | 2026-10-14 ~EST | desktop | trees: bitphoque | scope: Argent toolchain stood up + proven; extension layout settled (no bitphoque code yet). Cloned argent-lang/kcc20-reference to C:\kcc20-reference, cargo test --locked = 60/60 pass. Settled the sale extension: int price in sompi (0 idle / >0 listed), extension_commitment = BLAKE3(0x01 || LE64(price)) KCC1 §10 virtual element, 1-byte version tag for forward-compat; entrypoints collapse to set_price (owner-auth) + claim (payment-auth, no seller sig) alongside a byte-identical standard transfer. Filed layout + entrypoints into DESIGN; refreshed NOW with toolchain pins.
CONFIRMED: toolchain builds + asset passes in-environment (cargo test --locked, 60 passed 0 failed, incl conformance_borrowed_receive_vm + leader/delegate owner schemes); Rust 1.96.0 local clears 1.94.1 floor; pins argent 9a9f4b10 / rusty-kaspa a41a333b read from reference Cargo.toml.
GUESSED (flagged): price-per-UTXO merge is a wallet concern not a contract hole (believed, not tested); claim multi-token accounting sound as sketched (NOT yet designed — the open hard part); no-minter family indexer acceptance still unverified (carried from #017).
DRIFT: none material. The version byte was the only open layout choice and was taken; the real risk (claim accounting under leader/delegator) was deferred to its own pass, not folded into the layout.
prev-head: 6f3778e
---sig #019 | 2026-10-14 ~EST | desktop | trees: bitphoque | scope: claim design settled (no code); design phase complete. Resolved the last gate: claim is SINGLETON (one listed token/tx), dissolving the batched-claim double-count + coverage-gap bug class under leader/delegator. Batched claim deferred (clean seam). Specified set_price + claim invariants, checked six anti-theft/anti-grief vectors. Filed to DESIGN; NOW moved to build (scaffold Argent project).
CONFIRMED: singleton claim uses only primitives already read in the reference (require_owner_authorized, ScriptPubKeyP2PK, OpCovInputCount, become-continuation, sig #017); no unproven builtin needed.
GUESSED (flagged): the six vectors are reasoned, NOT VM-tested — become rejection tests once bitphoque.ag exists. price-per-UTXO merge = wallet concern (untested). no-minter family indexer acceptance unverified (carried from #017).
DRIFT: none. Claim difficulty was isolated to batching and removed by the singleton constraint; layout untouched, as predicted in #018.
prev-head: 870430f

<<<END LEDGER>>>
