# BitPhoque — Status

_Read `operating_baseline.md` (HOW) alongside this (WHERE). Stable reference — ABI, consensus rules, contracts, ground facts, file tree — lives in `bitphoque_reference.md` (lookup only). On-chain tx ledger + session narrative in `bitphoque_history.md`. KCC20 redesign intent in `kcc20-findings.md`. BitPhoque lives in `C:\bitphoque` with its own git repo, separate from oracle-protocol and gen5._

---

## CANONICAL PATHS

- Project root: `C:\bitphoque` (own git repo)
- Docs: `C:\bitphoque\docs\`
- Rust harnesses canonical: `C:\bitphoque\rust-examples\`
- Status doc: `C:\bitphoque\docs\bitphoque_status.md` (this file)
- Baseline: `C:\bitphoque\docs\operating_baseline.md`
- Reference: `C:\bitphoque\docs\bitphoque_reference.md` (pins block added 2026-10-14; `.sil` architecture sections bannered superseded — see DESIGN)
- KCC20 findings: `C:\bitphoque\docs\kcc20-findings.md` (SUPERSEDED banner 2026-10-14 — recorded 4-field/minter/gen5-join shape out of scope; see NOW/DESIGN)
- Argent toolchain (reference clone): `C:\kcc20-reference` (argent-lang/kcc20-reference @ c8a0871; `cargo test --locked` 60/60 — re-proven on pyt 2026-10-14)
- BitPhoque Argent project: `C:\bitphoque-argent` (SCAFFOLDED sig #022 — vendored verbatim from `C:\kcc20-reference`; package `bitphoque-argent`; `kcc20.ag`->`bitphoque.ag`; `set_price`+`claim` on the KCC20 actor; `cargo test --locked` 60/60. NOT yet git-init'd — robocopy excluded `.git`)
- Migration record: `C:\bitphoque\docs\bitphoque_migration_to_pyt.md`
- Retired TN12 tree: `C:\_RETIRED_kaspa-tn12` — NOT migrated to pyt (never came over; the "delete after CLI migration" item is auto-resolved here)

**MACHINE:** development moved from KarrrlsBigDaddy to **pyt** on 2026-10-14 (both Windows, same `C:\` paths). big daddy retains gen5. Every path in this block is pyt-local.

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

**State (2026-10-14, on pyt):** BUILD IN PROGRESS — entrypoints + rejection suite green, 73/73 (sig #023). Development migrated to pyt this session; toolchain re-proven and repo verified intact (sig #021). BitPhoque = standalone single-covenant KCC20 asset in Argent on argent-lang/kcc20-reference `kcc20.ag`. Fixed supply 42,069, pubkey-owned (0x00), on-chain sale. Toolchain re-proven on pyt (C:\kcc20-reference, cargo test --locked 60/60). Full design settled (see DESIGN): six-field state; sale = int price sompi (0 idle/>0 listed) in extension_commitment = BLAKE3(0x01 || LE64(price)); set_price + claim, BOTH singleton, alongside a byte-identical standard transfer; genesis pre-mints full supply, no minter. DONE sig #022: scaffolded C:\bitphoque-argent (vendored verbatim, package + kcc20.ag->bitphoque.ag renamed), authored ext_commitment + set_price + claim on the KCC20 actor. DONE sig #023: sale rejection-test suite (tests/sale.rs, 13 tests) — cargo test --locked now 73/73 (60 conformance + 13 sale); the six security vectors are VM-tested. Not yet: exercise on TN10, genesis.

**Mainnet caution (unchanged):** KCC-20 Last Call/Draft-in-file/NOT Final; ABI not converged (kccs #28). Conform + exercise on TN10 now; HOLD genesis until Final + converged; re-diff if spec moves. Composability gate narrowed (sig #021): the reference is SPEC-tagged (leader transfer `79c71c23` / delegator `fd3ef14a`), NOT kcc20-live (`c3d9f92f`) — standard transfer/delegator likely decode on live tooling; the leader/borrowed-receive path won't until #28 converges. One open check: which leader tag do kcc20.info / kaslab decode (`79c71c23` vs `c3d9f92f`).

**Toolchain pins (now recorded in reference.md):** argent + argent-runtime @ argent-lang/argent 9a9f4b10; rusty-kaspa a41a333b (must match TN10 for deploy); Rust 1.94.1 floor (pyt local 1.99.0). Reference repo = argent-lang/kcc20-reference @ c8a0871 (2-commit repo; the state tested 60/60).

**Single next action (next session):** exercise set_price + claim on TN10 against the target pin — genesis a listed BitPhoque UTXO, drive a real reprice/cancel and a real buyer-pays-seller claim on-chain, and confirm the covenant accepts/rejects as the VM tests predict (the tests use synthetic UTXOs, not a live node). Also decide claim's claimed-output borrow_guard (preserve-vs-zero) before genesis. (Sale suite landed sig #023; cargo test --locked 73/73.)

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
- **Merge is per-commitment-class (CORRECTED sig #021, read in the reference transfer).** The standard transfer forces ONE shared extension_commitment across all inputs AND outputs (`require(delegates[i].extension_commitment == extension_commitment)` on inputs, same check on outputs). So idle tokens (all share BLAKE3(0x01||LE64(0))) merge freely; two listed tokens merge only at the SAME price; idle+listed and different-price listed merges are REJECTED by the transfer, not collapsed. Each distinct price is its own merge-fungibility class — a wallet constraint to build to, not a hole. Supersedes the "keeps one price on the whole" phrasing in the bullet above; becomes a Phase-2 rejection test.

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
- [~] **Build BitPhoque as single-covenant KCC20 (Argent).** Base = reference kcc20.ag->bitphoque.ag, extended with set_price/claim over the extension_commitment virtual element carrying the int price. Sub-steps: (a) [x] Argent toolchain stood up + pinned, reference compiles (sig #018/#021); (b) [x] idle vs listed commitment layout settled (sig #018); (c) [x] set_price + claim authored on the KCC20 actor, compile + 60/60 (sig #022); (d) [ ] exercise on TN10 against the target pin. Remaining: TN10 exercise (rejection-test suite done, sig #023).
- [x] **BitPhoque rejection-test suite — DONE (sig #023).** tests/sale.rs (13 tests), cargo test --locked 73/73. VM-tested: set_price list/reprice/cancel + reject non-owner + reject negative price; claim accept + reject underpay / missing payout / wrong-address payout / open-below-commitment / unlisted / stays-listed / partial-fill / second-input bundling (singleton); plus a transfer-sweep-needs-owner test. Low-level builder (own redeem + sig scripts) so malformed cases reach the VM.
- [ ] **Decide claim's claimed-output borrow_guard (sig #022).** Currently carried unchanged from the seller (inert under BORROW_DISABLED); preserve-vs-zero is genesis-irreversible — settle before genesis.
- [ ] **Fold the silverscript pin into reference.md (sig #022).** Argent lowers to kaspanet/silverscript @ 3ed9733 (v1.0.0); the ScriptPubKeyP2PK/P2SH builtins live there. Add it to the pins block alongside argent 9a9f4b10.
- [x] **Argent toolchain setup + pin — DONE (sig #018/#021).** argent + argent-runtime 9a9f4b10, silverscript 3ed9733 (v1.0.0), rusty-kaspa a41a333b pinned; cargo test --locked 60/60 on pyt. (silverscript pin still owed into reference.md — see below.)
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
- [x] **kcc20-findings.md superseded — DONE 2026-10-14 (sig #021).** SUPERSEDED banner added at the top of the file pointing to NOW/DESIGN; the recorded 4-field / scheme-0x02 / isMinter / KCC20Minter-controller / gen5-join shape is flagged out of scope. Body kept as historical record.
- [ ] **Verify indexer acceptance of a no-minter pre-minted KCC20 family** (kcc20.info / kaslab) — the reference assumes PublicMint/TokenSeed; confirm a direct-genesis fixed-supply token decodes. Composability gate.
- [ ] **Hold mainnet genesis until KCC-20 is Final + ABI converged** (kccs #28). Re-run the shape diff vs the ratified spec before genesis.
- [ ] Push: local master ahead of origin (HEAD now da3798e). Run the secret-sweep (`.key` files are gitignored but confirm nothing sensitive is staged), then push. NOTE: the pyt migration already provides an off-machine copy (repo now on big daddy + pyt), so the "get work off the single machine" risk is reduced; the secret-sweep + push remains for a remote backup.
- [~] **reference.md partial sharpen — 2026-10-14 (sig #021).** Added: pins block + staleness banner flagging the `.sil` architecture sections (STRATEGIC CONTEXT / THE CONTRACTS / CONSENSUS ABI) as superseded by DESIGN. Node/VPS/keys/deployment facts kept (still valid, but node/CLI are big-daddy-local — re-establish on pyt). STILL OWED: full six-doc restructure + rewrite of the stale architecture sections to the Argent/KCC20 design.
- [ ] Migrate status/reference/history to full marker-zone + six-doc structure (this rebuild started it for status; reference + deployments.md still owed; no deployments.md exists yet).
- [ ] cPanel: evaluate whether still needed (outlawpolitics.org redirect).
- [~] D: build-clone bin sweep (recopy 14 harnesses → clone, rebuild). — big-daddy-only; no `D:` build-clone on pyt. Moot here; rebuild fresh on pyt if/when the harnesses are needed.
- [x] Delete `C:\_RETIRED_kaspa-tn12` — resolved on pyt (never migrated; absent here). Still present on big daddy if you want it gone there.
- [ ] Buy transactions don't record seller as counterparty in tx history.
- [ ] index.html 'get started' section — small copy updates (receive/send done; rest reference old flows).
- [x] **reference.md pin block — DONE 2026-10-14 (sig #021).** Pins block added to reference.md: argent + argent-runtime @ argent-lang/argent 9a9f4b10, rusty-kaspa a41a333b, Rust 1.94.1 floor, reference repo @ c8a0871, re-proven 60/60 on pyt. PIN-ASSERT-for-Argent note included.

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
---sig #020 | 2026-10-14 ~EST | desktop | trees: bitphoque | scope: session close at the design/build boundary (no code). Design phase complete through #019. Decided: BitPhoque Argent project = NEW repo at C:\bitphoque-argent (standalone, keeps the docs repo docs-only). Starting-point (vendor kcc20.ag verbatim vs depend-as-crate) left undecided — settle at scaffold. Flagged reference.md pin debt into OPEN ITEMS.
CONFIRMED: nothing new built or run this session; state matches #019 (toolchain proven 60/60, design settled). HEAD opened da3798e, tree clean, master 8 ahead of origin.
GUESSED: none load-bearing added.
DRIFT: intended to scaffold; user closed at the design boundary instead — correct seam, nothing half-built. One mechanical catch this session: a DESIGN edit anchored across a non-contiguous bullet span (the Standard-transfer bullet sat between the two targets); re-issued as two single-bullet edits. Lesson for marker-zone edits: anchor one bullet at a time, never assume adjacency.
prev-head: da3798e
---sig #021 | 2026-10-14 ~EST | desktop (pyt) | trees: bitphoque | scope: MIGRATION KarrrlsBigDaddy → pyt + docs clean-close; earlier upstream findings folded. Moved BitPhoque to a dedicated machine (pyt) to end gen5/bitphoque resource contention. Stood up toolchain on pyt (git 2.55, rustup/cargo 1.99.0, VS Build Tools MSVC), re-cloned argent-lang/kcc20-reference → C:\kcc20-reference, cargo test --locked = 60/60 re-proven. Moved C:\bitphoque via tar+USB (SMB auth failed; SSD flaked), verified intact over the device bridge. Folded the earlier upstream re-derivation: reference is SPEC-tagged (leader transfer 79c71c23 / delegator fd3ef14a), NOT kcc20-live (c3d9f92f); vendor decision forced (publish=false/bin-only); merge is per-commitment-class. Updated NOW + DESIGN; added reference.md pins block + staleness banner; bannered kcc20-findings.md superseded; reconciled open items.
CONFIRMED: cargo test --locked 60/60 on pyt (MSVC host). Repo verified over bridge — full tree, docs byte-exact (status.md 23202B), .key files present (owner/v2_owner/buyer1-3), .git complete (objects/logs/refs); committed HEAD = da3798e (sig #019). Reference read at argent-lang/kcc20-reference c8a0871: 6-field KCC20State [int,byte32,byte,byte,byte32,byte32] 112B; transfer tag 79c71c23 / delegator fd3ef14a; transfer enforces shared extension_commitment + output amount>=0 + conservation; pins argent 9a9f4b10 / rusty-kaspa a41a333b in Cargo.toml.
GUESSED (flagged): kcc20.info/kaslab indexer template (79c71c23 vs c3d9f92f) still unverified — the one remaining link in the composability gate. .key files present on pyt but decrypt/import NOT yet verified — do before wiping big daddy. The six claim rejection vectors still reasoned, not VM-tested (become tests at scaffold).
DRIFT: SMB copy blocked (error 1326 then 86 — big daddy PIN/MS-account auth); fell back to tar+USB after the SSD flaked (connection in/out). Retracted an in-session error — "reference moved off pin 9a9f4b10" conflated argent-lang/argent (toolchain pin, lives in the reference Cargo.toml) with argent-lang/kcc20-reference (the repo, 2-commit @ c8a0871); no re-pin owed. sig #020 was stranded uncommitted (status.md working tree carried #020 while HEAD stayed at #019/da3798e); this commit captures #020 + #021 together. No shell on pyt via the bridge — the git commit itself is run by the user.
prev-head: da3798e
---sig #022 | 2026-10-14 ~EST | desktop (pyt), via Claude device bridge (no shell on pyt) | trees: bitphoque, bitphoque-argent | scope: FIRST CODE — scaffolded the Argent project and authored BOTH sale entrypoints; compiles 60/60. robocopy-vendored C:\kcc20-reference -> C:\bitphoque-argent (excl target/.git); renamed package -> bitphoque-argent in Cargo.toml + Cargo.lock; renamed kcc20.ag -> bitphoque.ag + fixed token_seed.ag import. Authored ext_commitment(price)=BLAKE3(0x01||LE64(price)) helper, set_price (owner-auth list/reprice/cancel, singleton) and claim (payment-auth atomic purchase, singleton, no seller sig) directly on the KCC20 actor; transfer/transfer_delegator left byte-identical. User ran each cargo test --locked gate (60/60 at verbatim copy, after rename, after both entrypoints).
CONFIRMED (read at argent 9a9f4b10 + silverscript 3ed9733 this session): conformance survives added entries — dispatch tag = BLAKE3(entry_signature)[0:4] per-entry (tests.rs kcc20_dispatch_tags_match_spec_vectors), and the ABI/vector tests check only NAMED entries non-exhaustively, so transfer/delegator keep 79c71c23/fd3ef14a and the state layout is untouched. Singleton is FREE: a leader entry with no `consumes` clause compiles to require(OpCovInputCount==1) (argent emitter.rs; Rule 3, leader-delegate-input-groups.md). ScriptPubKeyP2PK IS a silverscript builtin (builtin_types.rs: 36-byte SPK, ctor takes a pubkey; expression.rs lowers to 0x0000||OpData32||pk||OpCheckSig) — claim's seller-payout check needs no hand-built script. tx.outputs[i].scriptPubKey->OpTxOutputSpk, .value->OpTxOutputAmount. silverscript v1.0.0 (3ed9733) is the SIL layer argent lowers to (argent Cargo.toml). LF-only status.md confirmed.
GUESSED (flagged): the six security vectors are reasoned, NOT VM-tested (become the rejection suite). claimed-output borrow_guard carried unchanged from seller (inert under BORROW_DISABLED) — preserve-vs-zero open. no-minter family indexer acceptance unverified (carried from #017). template_hash/covenant-id now differ from the vanilla reference — EXPECTED (added entries change the template; re-genesis derives a fresh id anyway).
DRIFT: none material. Proved the conformance thesis by reading the tests before authoring rather than assuming it; isolated the one real unknown (P2PK payout) and grounded it in the silverscript source instead of hand-rolling script bytes. No shell on pyt via the bridge (computer-use terminals are click-tier, typing blocked) — user ran every cargo gate; this ledger + commit are likewise user-run. bitphoque-argent not yet git-init'd (robocopy excluded .git).
prev-head: 0305962
---sig #023 | 2026-10-14 ~EST | desktop (pyt), via Claude device bridge | trees: bitphoque-argent | scope: BitPhoque sale rejection-test suite — the six security vectors are now VM-tested, not just reasoned. Added src/bin/kcc20/tests/sale.rs (mod sale; in tests.rs), 13 tests driving set_price/claim through the covenant VM at the low level (own redeem + signature scripts, so malformed cases reach the VM rather than builder pre-checks). cargo test --locked = 73/73 (60 conformance + 13 sale).
CONFIRMED (VM-proven this session): set_price accepts list(0->P)/reprice(P->P')/cancel(P->0), rejects a non-owner signature and a negative price. claim accepts a correctly-opened, correctly-paid purchase; rejects underpay (output value < P), missing payout, payout to a wrong P2PK address, opening below the commitment (ext_commitment(opened) != commitment), an unlisted/idle token, a still-listed continuation (become forces ext_commitment(0)), partial-fill (amount conserved), and a bundled second BitPhoque input (singleton OpCovInputCount==1). A listed token still cannot be swept via transfer without the owner signature. Rust ext_commitment = blake3(0x01 || price.to_le_bytes()) matches the contract's (price as byte[8]) -> OpNum2Bin(8). P2PK payout SPK = version-0 OpData32||pubkey||OpCheckSig matches new ScriptPubKeyP2PK(pubkey(owner)).
GUESSED (flagged): claimed-output borrow_guard still carried unchanged from the seller (preserve-vs-zero open). no-minter family indexer acceptance unverified (carried). TN10 on-chain exercise still owed — VM tests use synthetic UTXOs + a fixed covenant_id, not a live node.
DRIFT: none. Mapped the existing Transfer/run_vm harness before writing; the sale entries needed a purpose-built low-level builder (different arg shapes + a non-covenant P2PK payout output), not the Transfer struct. No shell on pyt — user ran the cargo gate (73/73); this commit is user-run.
prev-head: a704f5b

<<<END LEDGER>>>
