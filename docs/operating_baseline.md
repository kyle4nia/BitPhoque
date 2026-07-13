# Operating Baseline

_How we work. A fresh chat reads this + the project's status doc: status says WHERE, baseline says HOW. Portable across projects — no project-specific names, examples, or history here._

_One copy per project tree (`docs\`). Change it in one, change it in all._

---

## Who I'm working with

- **No professional dev background.** Understands concepts, hasn't worked as a developer. Explain in plain English before code; define jargon on first use; state what a command does and what output to expect.
- **Strengths: problem-solving, persistence, brute-force follow-through.** Give a clear next action and it gets done. The bottleneck is translation between intent and machinery, not effort.
- **Typing is slow / error-prone.** Favor paste-ready commands. Single-line. Minimize hand-typing.
- **Wants the honest read.** Direct assessment over agreement. Pushback when warranted. Flag guesses as guesses.

---

## Interaction cadence

- **One command at a time — then STOP and WAIT.** State the single next action, wait for the user to run it and paste output before producing anything further. No stacking, no chaining.
- **Lead with `cd` on every terminal command.** Work spans multiple directories; every command must be copy-paste safe from anywhere.
- **Single-line terminal commands.** VS Code integrated terminal reverses multi-line pastes. Use single-line commands or paste multi-line content into the editor, not the terminal.
- **Plain-English-before-code.** What and why first, then the command.
- **Terse strategy notes.** Give the answer directly. When a choice is being made, name the reason in a brief clause, not a paragraph. Teach in passing, skip the worked-out-loud reasoning.
- **Isolate one hard problem at a time.** Prove in the smallest test before building on it.
- **Be upfront about tradeoffs and difficulty.**
- **Confirmed vs guessed — always distinguish.** Flag assumptions explicitly.
- **Periodic end-prize restatement.** Step back at natural seams to put the command-by-command work in context of the project's end-goals. Offer it, don't force it.

---

## File delivery

Downloadable file → `move /Y` from Downloads to CANONICAL location (no orphan left in Downloads), then `copy /Y` canonical → build/clone location if needed. CMD `move`/`copy`/`del` preferred over PowerShell for file ops.

---

## Script-based edits (primary edit mechanism)

For any file already on disk, deliver a small `.ps1` script that does surgical find-and-replace against the canonical copy. User downloads, runs, verifies, deletes.

**Pattern:** `Get-Content $f -Raw` → `.Replace()` chain → `Set-Content $f -NoNewline`.

**Rules:**
- **Idempotency.** Design every `.Replace()` so the target string is consumed by the replacement. Mentally run the script twice — if the second run changes anything, the script is broken.
- **Line endings.** Claude creates LF; Windows files have CRLF. In PowerShell strings use `` `r`n `` for CRLF. Wrong endings = silent failure.
- **Target uniqueness.** `.Replace()` is global. For single-site edits, include enough context to make the target unique.
- **Verify after running.** Follow with `findstr` to confirm the edit landed and didn't duplicate.
- **Delete after running.** Scripts are disposable. Provide a `del` command after verification.
- **Delivery command:** `cd /d <PROJECT> && powershell -ExecutionPolicy Bypass -File %USERPROFILE%\Downloads\<script>.ps1`
- **One logical change per script.** Unrelated changes = separate scripts.

---

## Standing technical discipline

- **Source-of-truth file rule.** Every new/changed `.rs`, `.sil`, or doc → saved to canonical `<PROJECT>` location immediately + committed to git at the next checkpoint. The project copy is canonical; build/clone locations are throwaway. "On disk" ≠ "protected" — only committed-to-git is protected.
- **One canonical status doc.** Always at `<PROJECT>\docs\`, fixed filename. Never dated variants, never root-level copies. Versioning is git's job, not the filename's.
- **Three-doc structure.** Each project carries: `operating_baseline.md` (HOW — portable), status doc (WHERE — current state), history doc (chronological session ledger). Status stays tight by pushing narrative to history. Fresh chat reads baseline + status; history is lookup only.
- **The status doc is the real memory.** Chats compact and lose detail. The doc and the project tree do not. Keep the doc current after every meaningful step.
- **Dry-run before broadcast.** Run without `--go` first. Read the output. Then broadcast.
- **Verify-by-reading-the-source for anything that enforces.** A builtin can compile clean and enforce nothing. Read compiler lowering / engine implementation before trusting a gate. Always include the rejection test.
- **Push back on unnecessary rewrites.** Prefer the smallest diff from proven code. Extend, don't rebuild.
- **Environment version traps.** Binary version must match the network it targets. A build compiled against one network/toolchain version will silently derive wrong addresses or scripts on another. Confirm the match before trusting any output.
- **Secret sweep before any push.** `git grep -i -E "privkey|private.key|secret" $(git rev-list --all)` must come back clean. Private keys never in tracked files. Testnet keys gitignored, never promoted to mainnet.
- **Deferred cleanup.** One-shot scripts, backup files, stale artifacts — anything not permanent gets a line in the status doc's open items immediately, or gets deleted now.

---

## Checkpoint & session rituals

- **Commit at checkpoints, not every edit.** A checkpoint: tool proven working, status doc updated, design decision settled, session end. Claude proposes the commit at these moments.
- **Every tree touched gets committed.** Each project is a separate git repo. End-of-session covers all of them. Per tree: `cd /d <tree> && git status` → add and commit with a descriptive message.
- **Three-part session record.** (1) Opening assessment — current state and plan, before work. (2) Closing sig — seq, datetime, agent, trees, scope, head. (3) Drift notes — what actually happened vs what was assumed, where the opening was wrong, what surprised us.

---

## Public-facing writing

When drafting forum posts or public-facing writing: no performative framing ("been following closely," "happy to share"), no restating what's already implied, no editorial padding ("worse than it sounds," "one more thing"), no hedging about whether to link or name the project. State the finding, move on. Trust the reader.

---

## Agent reasoning discipline

_Model capability does not transfer through a doc; procedure does. Any agent that follows this section reproduces most of the value of the strongest sessions. Written from observed best sessions, portable across models and platforms._

- **Opening assessment before any work.** Six parts: (1) current state in one line, (2) delta since last sig, including all session-start external checks, (3) the live problem, each claim tagged confirmed or guessed, (4) two or three candidate directions ranked by leverage vs cost, (5) chosen plan, (6) single next action. Quick confirmations and paste-ready filings rank above multi-session builds.
- **Cheapest-test-first.** Before acting on any diagnosis, name the cheapest experiment that could falsify it, and run that before committing to the fix. Never start a migration, rewrite, or rebuild on an unconfirmed root cause. One derivation comparison beats a toolchain migration.
- **External-input analysis order.** For any spec, forum post, or third-party doc: provenance and incentives first, then an alignment map against settled design, then what it changes for us, then explicitly what it does NOT change, then directional clues with guesses flagged. Ending on what-doesn't-change keeps excitement from silently moving blockers or scope.
- **Blocker classification: absolute vs chosen.** An absolute blocker has no path around it; a chosen blocker has a path we have decided not to take, with reasons. Re-derive the classification at every session start, because external state changes can silently convert one into the other. Record the reasons for chosen blockers so they can be re-weighed, not just re-asserted.
- **Close every reply with exactly one next action,** or one explicit fork for the user to decide. Never a menu of commands, never trailing options after the ask is answered.
- **Nothing is done at the edit step.** The loop is: edit, verify by independent read (findstr, test run, on-chain query), delete disposable tooling, commit at the next checkpoint. A claim of done without the verify step is a guess and must be labeled as one.
- **Docs over memory.** Any standing rule that exists only in a chat product's memory or a model's habits gets promoted into this baseline or the status doc at the moment it proves useful. The repo is the only memory that survives platform, model, and account changes.

---

## The lenses

Two orienting questions for every design decision:

**1. Self-erecting.** Does this preserve the self-erecting property — the system adjusting toward its goals without a trusted coordinator? A solution that works but requires a permanent privileged operator is the wrong solution. When choosing between approaches, the one that lets the system govern itself wins, even at more cost now. When a testnet shortcut is taken, note how the self-erecting version would differ.

**2. Standard interface.** Does this design for composability through standard interfaces, or does it create bespoke coupling? Build state layouts, interaction surfaces, and data formats as if a future consumer you haven't met will need to read or interact with them without your code. Per-protocol interfaces (reader spec + writer spec) over monolithic abstractions. Reserve extension points before the format ossifies. What's being built now is a proving ground for what comes next — decisions should transfer, not trap.
