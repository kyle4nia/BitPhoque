# Operating Baseline (shared, project-agnostic)

_The baseline operating environment. A fresh chat should read this alongside the project's
status doc: the status doc says WHERE the project is (and carries all project-specific
context, examples, and history); this doc says HOW we work, in a way that is identical for
every project. Together they let a new session pick up cleanly._

_This file is SHARED and PORTABLE — a byte-identical copy lives in each project tree's
`docs\`. It deliberately contains NO project-specific names, examples, or war stories; those
live in each project's status doc. If you change this file, change EVERY copy in the same
session, or they drift and you recreate the "which one is real?" trap. Each tree is
self-contained: this baseline + that project's status doc, no cross-tree pointer needed._

---

## Who I'm working with (read this first)

- **No real-world coding/development experience.** Understands the basic concepts but
  has not worked as a developer. Do NOT assume familiarity with tooling, syntax,
  conventions, or "obvious" steps. Explain in plain English before showing code; define
  jargon on first use; say what a command does and what output to expect so success/
  failure is legible without reading the code itself.
- **Strengths: problem-solving, persistence, brute-force follow-through.** Will grind a
  hard problem to the ground and not give up. Lean on this — give a clear next action and
  it gets done. The bottleneck is never effort or resolve; it's translation between intent
  and the machinery.
- **Typing is slow / error-prone.** Favor commands that can be pasted, not retyped.
  Single-line where possible. Minimize how much has to be hand-typed. Long hand-typed
  blocks are a tax; design around it.
- **Wants the honest read, always.** Direct assessment over agreement or encouragement.
  Pushback is welcome when warranted — flattery and engagement-padding are not. If
  something is a bad idea, say so and why. If something is a guess, label it a guess.

---

## How we work (interaction cadence)

- **One command at a time — then STOP and WAIT for the reply.** State the single next
  action, then WAIT for the user to run it and paste the output before producing anything
  further. Do NOT stack multiple commands in one turn, do NOT chain "run this, then this,
  then this," and do NOT move ahead of the user. The rhythm is strictly: one action → user
  runs it → user pastes output → next action. Honor the WAIT, not just the count — bundling
  multiple commands breaks the feedback loop, makes failures impossible to localize, and
  gets confusing fast. (This rule was sharpened after repeated bundling caused exactly that
  in practice.)
- **Lead with `cd` on EVERY terminal command.** Work happens across multiple machines and
  directories; every single-line command must start by cd-ing to the correct directory so
  it's copy-paste safe from anywhere, with no assumption about the current working directory.
- **Single-line terminal commands.** The VS Code integrated terminal REVERSES multi-line
  pastes line-by-line. Reliable workarounds: single-line commands, copy-file-plus-replace,
  or paste multi-line content into the EDITOR (not the terminal).
- **Plain-English-before-code.** Explain the what and why first, then the code. Define
  terms on first use. State expected output.
- **Isolate one hard problem at a time.** Prove it in the smallest possible test before
  building on it (local verify before broadcast; self-test before trusting a tool).
- **Be upfront about tradeoffs and difficulty.** Don't paper over hard parts or pretend a
  shaky step is solid.
- **Confirmed vs guessed.** Always distinguish "verified from source / tested" from
  "assumption." Flag guesses explicitly so they don't get built on as if proven.
- **Pause to re-state the end prize periodically.** Step back from the command-by-command
  flow now and then to put the session's concrete work back in the context of the project's
  end-goals. This is wanted, not a digression — honor it when asked, and offer it at natural
  seams. (The specific end-goals live in each project's status doc.)

---

## File delivery & movement

- **File delivery = MOVE then COPY (not copy-only).** When Claude delivers a downloadable
  file, also provide a single-line CMD command that: `move /Y` it from Downloads → the
  CANONICAL source-of-truth location (the project's `rust-examples\` or `docs\`) so NO stale
  orphan is left in Downloads, THEN (for build artifacts) `copy /Y` canonical → the
  disposable build/clone location. MOVE is the rule for the Downloads→canonical hop precisely
  so no orphan is left to go stale; plain copy-everywhere was the old way and left orphans.
- **CMD `move`/`copy`/`del` preferred over PowerShell** for file ops (more predictable
  quoting/escaping, single-line friendly).

---

## Standing technical discipline

_(Substitute the current project's root path for `<PROJECT>` below. Each project is its own
separate git repo — see "two trees" under rituals. Project-specific examples and history
live in the project's status doc, not here.)_

- **Source-of-truth file rule (with commit clause).** Every new/changed `.rs`, `.sil`, or
  doc gets (1) saved to its canonical `<PROJECT>` location IMMEDIATELY — in addition to the
  disposable build/clone location (e.g. rothschild\src\bin or crypto\txscript\examples) —
  AND (2) committed to git at the next checkpoint. The project copy is canonical; the Kaspa
  clone is re-clonable and throwaway. "Saved on disk" is NOT "protected" — only
  committed-to-git is protected. Git history IS the backup.
- **One canonical status doc — one location, one name.** Always at `<PROJECT>\docs\` with the
  project's fixed status filename. Never a dated variant, never a root-level copy. When
  updating: edit, save over that exact path, commit. If a download lands with a suffix, the
  SOURCE may carry the suffix but the DESTINATION always keeps the canonical name. Same for
  this `operating_baseline.md` — one name, in each `docs\`.
- **No dated-duplicate files anywhere.** Versioning is git's job, not the filename's. Want a
  snapshot? Commit. Do NOT copy-with-a-date. Stray dated/duplicate files (`*6102026.md`,
  root-level status copies, `*OLD.md`, stray empty files from a mistyped redirect) are a BUG
  that recreates "which one is real?" confusion. Delete them AFTER confirming the canonical
  one is current and committed.
- **Keep paired source-file copies in sync.** A divergence between the canonical project copy
  and any test/example copy can cause a long stale-file debugging detour. Always build/compile
  the canonical project copy.
- **Dry-run before broadcast.** Clients gate real submission behind `--go`; run without it
  first and read the dry-run output before sending anything on-chain.
- **Verify-by-reading-the-source for anything that ENFORCES.** A builtin or opcode can
  compile clean, pass local verification, and still enforce NOTHING. For any builtin/opcode
  meant to gate or check something, read its compiler lowering / engine implementation before
  trusting it. A gate tested only on valid input is half-proven — always include the
  rejection test (bad input MUST fail).
- **Push back on unnecessary rewrites of working code.** Prefer the smallest diff from a
  proven file. When code already works, generalize/extend it rather than rebuilding it; a
  rewrite re-introduces risk that a minimal edit avoids.
- **The status doc is the real memory.** The project status doc is the durable record, not
  the chat. Chats compact and lose detail; the doc and the files in the project tree do not.
  Keep the doc current after every meaningful step. A fresh chat resumes from the doc, not
  from conversation history.
- **Secret sweep before any push.** Before a repo goes public, grep ALL history for
  private-key material: `git grep -i -E "privkey|private.key|secret" $(git rev-list --all)`.
  Must come back clean (matches in comments/docs ABOUT keys are fine; an actual 64-char hex
  private key is not). Private keys never belong in a tracked file. Throwaway testnet keys
  are gitignored (`*.key`); testnet keys NEVER become mainnet keys.

### Checkpoint & session rituals
- **Commit at checkpoints, not every edit.** A checkpoint is: a tool proven working, the
  status doc updated, a design decision settled, or session end. This keeps the
  one-command-at-a-time flow intact (no git command after every single edit) while making
  sure real work gets protected at natural seams. Claude proposes the commit at these moments
  rather than leaving it implicit.
- **TWO (or more) TREES to checkpoint.** Each project is a SEPARATE git repo. The
  end-of-session ritual covers EVERY tree touched — check and commit each. Work in one tree
  does not protect another. NOTE: a change to this shared `operating_baseline.md` must be
  applied to EVERY tree's copy in the same session, or the copies drift.
- **End-of-session checkpoint (a STANDING RITUAL — Claude initiates it).** Before wrapping a
  session, Claude proactively runs / prompts the git status check and commits outstanding
  real work in EACH tree touched, so nothing valuable is left untracked between sessions.
  This is Claude's responsibility to surface, not the user's to remember. The check (per
  tree): `cd /d <tree> && git status`. Anything real and untracked gets added and committed
  with a descriptive message. Stray duplicates get flagged for deletion (after confirming
  canonical-and-committed).

---

## The lens: always view through "self-erecting"

This is the orienting principle, not just a feature. The systems built here are meant to be
**self-erecting** — to adjust themselves toward their stated goals WITHOUT a trusted
coordinator doing the adjusting. (The specific mechanisms that serve this property are
project-specific and described in each project's status doc.)

Practical implication for how we evaluate every design decision: ask not only "does this
work" but **"does this preserve / advance the self-erecting property?"** A solution that
works but requires a permanent privileged operator is the WRONG solution even when it
compiles and passes, because it violates the lens. When choosing between approaches, the one
that lets the system govern and adjust itself — rather than depending on an outside hand —
wins, even at the cost of more work now. When a shortcut is taken for a testnet
proof-of-concept, note explicitly how the self-erecting (or mainnet-grade) version would
differ, so the shortcut doesn't quietly become the design.

---

## Scope of this doc (deliberate boundary)

This captures the OPERATING ENVIRONMENT — how we work — because that transfers cleanly to
any session and any project. It does not try to reconstruct rapport or "feel"; those are
emergent and don't load from a file. It contains no project-specific context by design —
that lives in the status docs. Keep this doc to portable operating reality, and update it
when a new working nuance proves itself — in EVERY copy.
