# Next Session Handoff — Housekeeping + Roadmap

_Load this AFTER `operating_baseline.md` and `bitphoque_status.md`.
This doc captures decisions and priorities agreed on 2026-07-11 for the
next working session. Delete after the session consumes it._

---

## Infrastructure decisions

- **PsychoNode is retiring.** VPS (Hetzner CX33, 89.167.32.239) is now the
  sole node infrastructure — mainnet and TN10. Cost regulation is the driver.
  Remove PsychoNode references from status doc and any operational docs that
  still point to it (Wake-on-LAN, addpeer, etc.).

- **VPS mainnet node status: CHECK FIRST.** Run the one-liner at session open
  to confirm the mainnet node is live and synced. If IBD isn't done or the
  service is down, fix before anything else — mainnet readiness is the
  prerequisite for everything in Phase 1.

---

## Session 1 scope: housekeeping

1. **Status doc cleanup** — the main deliverable.
   - Update "WHERE WE ARE" header date (currently says 2026-06-29, head is
     sig #010 from 2026-07-03).
   - Reorder session signatures into clean newest-first sequence (sigs
     #002–#006 are interleaved at the bottom).
   - Remove resolved items still listed as open in sig #008's carried-forward
     block (contract review items closed in sig #004/#009).
   - Remove or update PsychoNode references throughout.
   - Verify every open-item checkbox — if it's done, check it off; if it's
     stale, remove or reframe.

2. **Disk/repo cleanup** (if time permits).
   - Delete `C:\_RETIRED_kaspa-tn12` (file-handle release was pending).
   - D: build-clone bin sweep (recopy 14 canonical harnesses → clone).
   - Sweep for any stale downloads or one-shot scripts on Big Daddy.

---

## Banner goal: "Building a Covenant Token on Kaspa" write-up

**This is the highest-leverage move while V1 is unreleased.** A technical
write-up that walks through what BitPhoque solved — aimed at developers
entering the Kaspa covenant ecosystem. Not BitPhoque marketing; a
reference guide that happens to use BitPhoque as the worked example.

**Why it matters:**
- You are currently the only person who has built and proven a full
  covenant-based fungible token on Kaspa L1 (genesis → transfer → split →
  merge → list → claim, all live on TN10).
- When V1 drops and more developers arrive, whoever wrote the guide IS the
  ecosystem expert. First-mover on documentation is as valuable as
  first-mover on code.
- The BitPhoque repo is public. A companion write-up turns it from "someone's
  project" into "the reference implementation."

**Skeleton (to flesh out over sessions):**
- Why covenant-based vs indexer-based (KRC-20) — architectural tradeoffs
- The UTXO-per-balance model and conservation enforcement
- P2SH derivation for covenant UTXOs (template splice, Route B1)
- Sighash + signing in the browser (keyed blake2b, schnorr, ABI sigscript)
- Leader/delegate multi-input transactions
- Sale model: per-UTXO listing and trustless claim
- Parcel registry: discoverability without an indexer
- Lessons learned (the hard-won stuff: EngineFlags, ComputeBudget,
  CScriptNum encoding, address-index lag, send-to-self key mismatch)

**Format:** Start as markdown in `C:\bitphoque\docs\`. Could become a blog
post, a GitHub Pages companion to the repo, or a standalone tutorial site.
The content is the bottleneck, not the container.

---

## Roadmap phases (agreed framing)

| Phase | Gate | Focus |
|-------|------|-------|
| 0 — NOW | V1 unreleased | Infrastructure readiness, status doc hygiene, write-up, ecosystem presence |
| 1 — V1 DROPS | SilverScript V1 release | Compile v4 → mainnet genesis → deploy → announce. Same-day target. |
| 2 — POST-LAUNCH | BitPhoque live on mainnet | Polish, Oracle Protocol pivot, extract reusable modules |
| 3 — ECOSYSTEM | Ongoing | Covenant tooling, standards participation, identity as builder |

---

## SilverScript V1 baseline (session-open check)

As of 2026-07-11: 111 commits, zero releases, README still reads
"Experimental" and "valid only on Kaspa Testnet 12." No change from
sig #010 baseline. Report any delta at session open.
