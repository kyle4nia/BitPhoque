# BitPhoque — Migration to `pyt`

_Operational move doc. Source machine: `KarrrlsBigDaddy`. Target machine: `pyt`. Written 2026-10-14. Drop in `C:\bitphoque\docs\` and carry a copy to `pyt`._

---

## 1. Current status (what you're moving)

- **Phase:** design complete, build not started. Clean seam — nothing half-built to carry.
- **What BitPhoque is now:** a standalone single-covenant **KCC20** asset authored in **Argent**, built on `argent-lang/kcc20-reference`. Fixed supply 42,069, pubkey-owned (`owner_scheme 0x00`), no minter. On-chain sale lives in `extension_commitment` (`BLAKE3(0x01‖LE64(price))`, 0 = idle / >0 = listed) with two added entrypoints — `set_price` + `claim`, both singleton — alongside a byte-identical standard transfer/delegator.
- **Toolchain proven on `KarrrlsBigDaddy`:** `C:\kcc20-reference`, `cargo test --locked` = 60/60. Pins: argent + argent-runtime @ `9a9f4b10`, rusty-kaspa @ `a41a333b`, Rust 1.94.1 floor (1.96.0 was local).
- **Settled this session (CONFIRMED at the reference, c8a0871):**
  - Reference is on the **draft-spec** template — leader `transfer` tag `79c71c23`, `transfer_delegator` `fd3ef14a` — **not** kcc20-live (`c3d9f92f`). Confirms + names the genesis hold: standard transfer/delegator likely interoperate with live tooling; the leader/borrowed-receive path won't until kccs #28 converges.
  - **Vendor decision forced:** the reference is `publish=false`, bin-only, no `[lib]`, and covenant entrypoints can't be added externally → `bitphoque.ag` must be a copy of `kcc20.ag` extended with the two entrypoints. No crate-dependency option exists for the contract.
  - **Merge is per-commitment-class:** the standard transfer forces one shared `extension_commitment` across all inputs/outputs, so idle+listed and different-price listed merges are *rejected*, not collapsed. Becomes a Phase-2 rejection test.
- **Single next action (now on `pyt`):** scaffold `bitphoque-argent` — renamed verbatim copy of the reference, prove 60/60, rename `kcc20.ag → bitphoque.ag`, then add `set_price` + `claim` + the rejection suite.

---

## 2. Why migrate now (trigger)

- **Shared-resource contention.** BitPhoque and gen5 have always lived on `KarrrlsBigDaddy` and shared the same toolchain/node/environment. That contention has been a recurring issue and a standing risk — exactly the kind the environment-version-trap discipline warns about (one project's toolchain/node state bleeding into the other's).
- **gen5 owns `karrrl`.** The Claude Desktop instance (`karrrl`) is occupied running gen5. The Store (MSIX) build can't run a second data-separated instance and silently ignores local MCP servers, so a clean, independent BitPhoque instance on the same machine isn't practical.
- **The build phase needs isolation.** Scaffolding the Argent project stands up a pinned toolchain (argent/rusty-kaspa revs, Rust floor) and, for deploy, a TN10 node + rebuilt CLI. Running that beside gen5's environment is where contention would bite hardest. Isolating BitPhoque on `pyt` removes the whole class of problem.
- **Timing is right.** Design is complete (#019), toolchain is proven (60/60), scaffold hasn't started. Moving at this seam carries a proven, settled project and leaves nothing half-done behind.

---

## 3. What moves to `pyt`

Grouped by **method**, because how each thing moves matters more than the list.

### A. Direct copy — whole, with `.git` (preserves history + unpushed commits)

| Item | From (`KarrrlsBigDaddy`) | To (`pyt`) | Notes |
|---|---|---|---|
| **BitPhoque repo** | `C:\bitphoque\` (entire tree **including `.git`**) | `C:\bitphoque\` | Own git repo. Carries docs, `rust-examples\`, the bitphoque.org site/wallet JS, CLI, descriptor codec. Master is several commits **ahead of origin and unpushed** — a direct copy preserves those without exposing anything to a remote. |

The repo contains, at minimum (from CANONICAL PATHS):
- `docs\bitphoque_status.md` (WHERE), `docs\operating_baseline.md` (HOW), `docs\bitphoque_reference.md` (stale), `docs\kcc20-findings.md`, `docs\bitphoque_history.md`, `docs\bitphoque_mainnet_checklist.md`, `docs\bitphoque_swap_covenant_design.md`
- `rust-examples\` — the canonical Rust harnesses (~14)
- the site + wallet (`index.html`, wallet JS, descriptor codec) and CLI project

### B. Re-clone fresh from upstream (don't copy — re-prove on the new machine)

| Item | Source | To (`pyt`) | Gate |
|---|---|---|---|
| **KCC20 reference** | `git clone https://github.com/argent-lang/kcc20-reference` | `C:\kcc20-reference` | Then `cargo test --locked` → **must be 60/60**. This is your toolchain-reproves-on-pyt gate. Cargo pulls argent/rusty-kaspa at the pinned revs itself. |

Reference tree (for reference): `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `contracts/` (`kcc20.ag`, `public_mint.ag`, `token_seed.ag`), `src/bin/kcc20/`, `examples/`, `fixtures/`, `README.md`.

### C. Reinstall / re-provision (don't copy — install native on `pyt`)

| Item | How | Notes |
|---|---|---|
| **Rust toolchain** | `rustup` | 1.94.1 floor; `rust-toolchain.toml` in the reference/scaffold pins it. |
| **Git for Windows** | installer | Needed for Claude Code's Bash tool and `git`. |
| **Claude Code** | `irm https://claude.ai/install.ps1 \| iex` (PowerShell) | Sign in as **kyle4nia** → this is pyt's independent BitPhoque instance. Add `%USERPROFILE%\.local\bin` to PATH. |
| **Node.js** | installer (if site/wallet build tooling needs it) | Confirm whether the bitphoque.org tooling requires it; the browser wallet itself is static JS. |
| **TN10 node + CLI** | rebuild at the target pin | The CLI binary/network mismatch is already open — the CLI must be rebuilt against the target pin regardless, so do it fresh on `pyt`. |

### D. Secrets / keys — move out-of-band, direct transfer only, verify before wiping source

Enumerate and move these **by hand (USB or direct LAN), never via any remote or cloud.** Do not list their contents anywhere.
- **CLI `.key` files** — these have **no BIP-39 backup** (open item). Losing them is unrecoverable. Move them, verify they import on `pyt`, *then* consider the source copy.
- Any `.env` / config holding RPC endpoints, API keys, or node credentials.
- Encrypted wallet material / recovery phrases held on disk.

> This is also why a direct copy beats push-then-clone for the repo: the secret-sweep (open since #016) hasn't run, so pushing to origin now could expose unswept secrets. Direct transfer keeps everything off any remote until you've swept.

---

## 4. What does NOT move

- `C:\_RETIRED_kaspa-tn12` — retired, slated for deletion. Leave it; delete at source.
- **gen5 tree** — stays on `KarrrlsBigDaddy`. Separation is the whole point.
- `D:\` build-clone / rebuilt harness binaries — build artifacts; rebuild on `pyt`, don't copy.
- cPanel / `outlawpolitics.org` redirect — server-side, nothing to move.
- `C:\bitphoque-argent` — doesn't exist yet; it's **created fresh on `pyt`** (the scaffold), not migrated.

---

## 5. Transfer method (the PCs are 4 ft apart)

Proximity + unpushed commits + on-disk keys all point the same way: **move it locally, not through the cloud.**

- **Best:** a **USB external drive** hand-carried, or a **direct SMB share** between the two machines over the LAN. Either preserves git history and keeps secrets off every remote.
- `robocopy \\KarrrlsBigDaddy\... C:\bitphoque /E /COPYALL` over an SMB share handles the repo copy with attributes intact if you go the share route.
- **Avoid** git-push-then-clone for the repo until the secret-sweep has run (§3.D).
- The reference (§3.B) is the exception — it's public upstream, so re-clone it on `pyt` rather than copying.

---

## 6. Post-move verification gates (run on `pyt`, in order)

1. `C:\kcc20-reference` → `cargo test --locked` = **60/60**. If not, stop — toolchain/pin problem on `pyt`, fix before anything else.
2. `C:\bitphoque` → `git status` clean, `git rev-parse --short HEAD` matches the last LEDGER sig's head on `KarrrlsBigDaddy`.
3. Key files import/decrypt correctly on `pyt` (§3.D) **before** you remove them from the source.
4. Claude Code launches at `C:\bitphoque`, signed in as **kyle4nia**, can read `docs\bitphoque_status.md`.

Only after all four: begin the scaffold (§1, single next action).

---

## 7. Carry-over bookkeeping (file on the repo once `pyt` is live)

Two items from this session still owed to `C:\bitphoque\docs\bitphoque_status.md` (couldn't be written — session had no repo access):
- **Ledger sig #021** — the upstream code re-derivation (reference is spec-tagged `79c71c23`; vendor forced; merge is per-commitment-class). Full text is in the chat.
- **DESIGN merge-note correction** — different-price / idle+listed merges are rejected by the standard transfer, not collapsed. Full text is in the chat.
- Plus the still-open **`reference.md` pin block** debt (argent `9a9f4b10`, rusty-kaspa `a41a333b`, Rust 1.94.1 floor) — fold in on the next reference.md pass.
