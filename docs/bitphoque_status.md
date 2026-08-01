# BitPhoque — Status

_Read `operating_baseline.md` (HOW) alongside this (WHERE). Stable reference — ABI, consensus rules, contracts, ground facts, file tree — lives in `bitphoque_reference.md` (lookup only). On-chain tx ledger + session narrative in `bitphoque_history.md`. BitPhoque lives in `C:\bitphoque` with its own git repo, separate from oracle-protocol._

---

## SESSION-START CHECKLIST (any agent, any platform)

1. Read operating_baseline.md (HOW), then this doc (WHERE). Reference + history docs are lookup only.
2. Check https://github.com/kaspanet/silverscript for releases and README changes. Record the delta against the last recorded baseline in the opening assessment. Last recorded: 111 commits, zero releases, Experimental, TN12-only (2026-07-13).
3. Re-derive blocker classification (absolute vs chosen) per the baseline's reasoning-discipline section. Do not inherit last session's classification unexamined.
4. Deliver the six-part opening assessment before any work.

---

## WHERE WE ARE (2026-07-12)

Feature-complete spending wallet on TN10. Every layer proven live. Mainnet blocked on SilverScript V1 (experimental, zero releases).

**Current blockers:**
- **SilverScript V1 release** — no timeline. Everything mainnet-bound (re-genesis, v4 compile, downstream sync) is gated on this.
- **CLI import failures** — the only Cargo project with `bitphoque_cli` wired is `C:\_RETIRED_kaspa-tn12\rusty-kaspa` (v1.2.1-toc.3, TN12-era). All import attempts return NOT FOUND. Likely cause: TN12 SilverScript compiler derives different P2SH addresses than what's on TN10. Need to either confirm network match or wire the CLI into a TN10-compatible rusty-kaspa build.
- **`getUtxosByCovenantId` RPC** — feature request drafted, not filed on kaspanet/rusty-kaspa. Would eliminate the parcel registry as a hard dependency.

**Recent completions (sig #012, commit `4e32af4`):**
Descriptor removed from happy path (Send/Sell/Receive). Sender's wallet registers recipient's output parcel with registry on confirm. Receives recorded in tx history. "Show recovery phrase" feature. Supply accounting confirmed: 42,069 phoques across buyer1 (25,301) + buyer2 (1,001) + buyer3 (15,767).

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

---

## OPEN ITEMS

### Blockers
- [ ] SilverScript V1 release (gates all mainnet work)
- [ ] CLI binary/network mismatch — wire CLI into TN10-compatible rusty-kaspa build, or confirm TN12 build works against TN10 node
- [ ] File `getUtxosByCovenantId` issue on kaspanet/rusty-kaspa (draft ready)

### Mainnet path
- [ ] v4 downstream code sync (sigscript_v4 done; descriptor OK; compiled_real/CLI/wallet blocked on V1 compile)
- [ ] Clear `bitphoque_mainnet_checklist.md`
- [ ] Re-genesis produces new `covenant_id` — sweep `3baed5a3...` in all JS + harnesses
- [ ] CLI .key files have no BIP-39 backup — mainnet genesis should mint to BIP-39 wallet key

### Cleanup / minor
- [ ] cPanel: evaluate whether still needed (outlawpolitics.org redirect)
- [ ] D: build-clone bin sweep (recopy 14 harnesses → clone, rebuild)
- [ ] Delete `C:\_RETIRED_kaspa-tn12` (after CLI migration)
- [ ] Buy transactions don't record seller as counterparty in tx history
- [ ] index.html 'get started' section needs a pass of small copy updates (receive/send done; rest still reference old flows)

### Parked
- Auction — separate future build (stateful/temporal). See `bitphoque_swap_covenant_design.md`
- SMS/Twilio node monitor
- Price-escalating emission (step bonding curve)
- Key naming — legacy buyer1/2/3 rename sweep

---

## SESSION SIGNATURES

_Append-only. Newest first. Prior sigs pushed to `bitphoque_history.md`._

---sig #012 | 2026-07-11 ~EST | desktop | trees: bitphoque | scope: descriptor UX overhaul, registry-based discovery, tx history receives, show recovery phrase, supply accounting, explorer/RPC feature request draft | head: 4e32af4

---sig #013 | 2026-07-12 ~EST | desktop | trees: bitphoque | scope: kas-smith forum analysis (KCC20 spec, borrowed receive, open ICC/Capsule/virtual state), forum post shipped, CLI import debugging (TN12/TN10 mismatch identified), operating baseline trimmed, status doc trimmed | head: 6d603bf
OPENING ASSESSMENT: BitPhoque feature-complete on TN10, mainnet blocked on SilverScript V1. Plan: forum analysis, CLI sends from buyer keys.
DRIFT: CLI import universally failed — discovered the only wired Cargo project is TN12-era (v1.2.1-toc.3), likely deriving wrong P2SH addresses for TN10. Pivoted from "do the sends" to "diagnose why nothing imports." Forum work was productive — shipped a post to kas-smith sharing BitPhoque's production findings, analyzed KCC20 spec alignment, borrowed receive implications, and open ICC/Capsule architecture relevance to the oracle project.
