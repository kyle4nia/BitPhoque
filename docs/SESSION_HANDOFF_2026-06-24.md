# SESSION HANDOFF — 2026-06-24 (one-timer, read once then discard)

_This is a detailed catch-up for the chat picking up after a long multi-topic session.
The durable record is in `bitphoque_status.md`; this doc just carries the specific
decisions and live state from this session so nothing is lost in the handoff. Once you've
absorbed it and confirmed the status doc reflects the general track, this file can be
deleted._

---

## THE BIG MOVE: TN10 node relocated to Big Daddy

**What changed and why.** The TN10 node was running on **TestnetNode (.104)** and would
not sync — 3 days of grinding, reaching the tip then sliding back into header mode
(pruning-point thrashing). Root cause was judged to be **configuration cruft, not
hardware**: too many node instances had accumulated on .104 over time (alternating
launch scripts, stale/pre-Toccata datadir remnants, crossed wires). Notably .104 was the
*stronger* box but the *sicker* node, while **.206 runs the mainnet node flawlessly** on
the same-version v2.0.1 — the tell that the problem was setup, not silicon.

**Decision: rebuild the TN10 node fresh on Big Daddy (192.168.0.118)** — Kyle's primary
workstation: Ryzen 5900 (12c/24t), 128GB RAM, fresh NVMe (just clean-swept). Mainnet
node stays put on .206, untouched. A new dedicated node box with NVMe arrives ~2 days
from this session (2026-06-26ish) — Big Daddy may be interim or permanent; TBD.

**Hard-isolation discipline applied (Kyle's explicit, emphatic requirement):** zero
cross-talk between networks/versions. Each network gets its own folder, own binary copy,
own datadir (pinned via `--appdir`), own launch script, own pinned ports.

### The isolated TN10 tree on Big Daddy
```
C:\kaspa-testnet10\
    kaspad.exe              <- v2.0.1, confirmed `kaspad --version` = "kaspad 2.0.1"
    start-testnet10.bat     <- the ONE launch script
    datadir\                <- pinned via --appdir; fresh; created on first launch
```
Actual data path resolves to `C:\kaspa-testnet10\datadir\kaspa-testnet-10\datadir\`.

### The launch script (start-testnet10.bat)
Flags: `--testnet --netsuffix=10 --appdir=C:\kaspa-testnet10\datadir --utxoindex
--disable-upnp --rpclisten=0.0.0.0:16210 --rpclisten-borsh=0.0.0.0:17210 --yes`
- `--appdir` = the isolation linchpin (testnet node physically cannot touch mainnet data).
- `--utxoindex` kept SINGLE-PHASE per Kyle's call (he declined the two-phase
  sync-clean-then-add-index workaround; said not needed on Big Daddy). NOTE: there IS a
  documented rusty-kaspa defect (v1.1.0-rc.2 notes) where fresh-sync WITH `--utxoindex`
  can misbehave; the official fix is two-phase (sync without utxoindex, stop, relaunch
  with it). **If this node ever thrashes like .104 did, that two-phase dance is the
  fallback.** It did NOT thrash this session.
- `--yes` auto-confirms the DB-schema-version startup prompt so a fresh launch doesn't
  hang waiting on keyboard input.

### Sync status at session end — HEALTHY, the .104 wall is BEHIND us
The node **cleared the pruning-point proof cleanly** (validated all 21 levels 20→0 with
zero retries — the exact stage .104 kept failing). Then it entered forward sync and was
ripping through the pruning-point chain segment (~370 headers/sec, linear, no slides;
85k+ headers downloaded in ~4 min). The **Toccata guard** correctly rejected pre-Toccata
(version-1 header) peers up front and connected to protocol-version-10 peers. This is a
clean sync in progress; at session end it still needed time but was past the failure mode.
**Next chat: it may well be synced — check it.**

### IMPORTANT diag quirk during this sync stage
The diag (`bitphoque_node_diag`) read `blocks=0 headers=0` and ALL parcels NOT FOUND even
while the log showed 85k+ headers downloaded. This is NOT data loss — those headers are in
the pruning-point-chain-segment staging buffer, not yet committed to the DAG. The
committed counters jump 0→full-height in one move once the segment commits and validates.
So during this stage, **read the live kaspad LOG for progress, not the diag.** Use the diag
to confirm parcels only AFTER `synced=true`.

---

## IP REFERENCES — partial update, deliberate fail-loud strategy

The node moved, so hardcoded `const NODE` endpoints in the `.rs` clients are now stale.
Full inventory (17 files in `C:\bitphoque\rust-examples\`): most pointed at
`192.168.0.206:16210`, a few at `192.168.0.104:16210`. All now obsolete; the node is at
`127.0.0.1:16210` (localhost, since clients run on Big Daddy alongside the node).

**Decision: do NOT bulk-replace.** Kyle's instinct (correct): leave the stale strings,
let each binary **fail loud on first use**, and fix that one file then — one-thing-at-a-time,
and the connection error is an informative reminder of which binary is in play. Bulk-replace
would mask that signal.

**Only file updated this session:** `bitphoque_node_diag.rs` — `192.168.0.104:16210` →
`127.0.0.1:16210` (line 28). Canonical edited, copied to build clone
(`C:\kaspa-tn12\rusty-kaspa\rothschild\src\bin\`), rebuilt, ran, confirmed
`connected in 5 ms` to the local node. ✓

**Still stale (fix-on-use):** the other 16 `.rs` files. When you next run any of them
(claim, list, merge, inspect, genesis, transfer, etc.) and it can't connect, change that
file's `const NODE` to `127.0.0.1:16210`, copy canonical→clone, rebuild. Expect this on
first real `claim`/`list` against the new node.

### Build-wiring note (carried from earlier this session)
The diag/harnesses build from `C:\kaspa-tn12\rusty-kaspa\rothschild` as `--bin` targets.
That clone is a **tn12-flavored** rusty-kaspa checkout; `silverscript-lang` was added as a
path dep (`cargo add silverscript-lang --path C:\kaspa-tn12\silverscript\silverscript-lang`)
so harnesses importing `silverscript_lang` compile there. A fresh clone loses this — re-add.

---

## STILL-OPEN THREADS (carried, not resolved this session)

1. **Confirm the `28a90bc9…` claim mined.** The live CLI claim (buyer2→buyer1, 100M sompi,
   5000 phoques from parcel `4b0c8187…:0`) was node-ACCEPTED but never confirmed mined —
   .104 never synced. On the freshly-syncing Big Daddy node, once `synced=true`, check
   whether `4b0c8187…:0` flips to NOT FOUND (= claim mined) and buyer2's new 5000
   not-for-sale parcel appears. **NOTE:** this is a fresh datadir on a different box; the
   mempool-accepted-but-unmined tx may or may not have made it into a block — verify, don't
   assume.
2. **`bitphoque_history.md` still needs claim txid `28a90bc9…` added** — gated on
   confirming it actually mined (don't ledger as confirmed while unmined).
3. **Whole-parcel `send --go`** — VM-proven, never broadcast. Still pending.
4. **SMS node-monitor (Twilio) — STARTED, NOT BUILT.** Decided: PowerShell health script
   per node box, run every 2 min by Task Scheduler, checks process-alive + block-height-
   advanced (20-min-flat = stall), texts via Twilio (~$1/mo, reliable). Needs: a
   lightweight height-read method (the heavy Rust diag is too heavy for a 2-min cron) —
   leaning toward adding a `--rpclisten-json` listener to the node launch so PowerShell can
   `Invoke-RestMethod` the height. NOT built yet; node-health came first. Pick up after the
   node is synced and worth monitoring.

---

## CHECKPOINT STATE (commit these in next session if not done)
- `bitphoque_node_diag.rs` IP change (canonical) — **needs commit** to the bitphoque repo.
- 6 parked design notes — committed this session (`5dad2d2`): full block in
  `bitphoque_swap_covenant_design.md`, pointer stub in `bitphoque_status.md`. ✓
- The `start-testnet10.bat` lives in `C:\kaspa-testnet10\` (its own tree; not part of the
  bitphoque git repo — decide if you want it tracked somewhere).
