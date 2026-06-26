# BitPhoque — Full-Stack Security Audit

_Audit date: 2026-06-25. Method: history-blind source reconstruction. Scope: full stack
(web wallet, vendored crypto, in-browser P2SH derivation, v3 covenant, genesis establish,
descriptor codec). Status/baseline docs were read ONLY at the reconciliation stage, after
all findings were derived independently from source._

---

## Method note (why the findings are trustworthy)

Each layer was reconstructed cold — what the code actually does, derived from the artifact
itself — before the project's own `bitphoque_status.md` / `operating_baseline.md` were
opened. Findings were formed against ground truth (the source, the compiled JSON, the
on-chain vectors the code asserts), not against the project's framing of its own work. The
status doc was consulted last, purely to reconcile claimed-vs-actual. This ordering is what
makes the agreement between audit and docs meaningful rather than circular: the no-minter
supply mechanism, the i64 width question, and the witness-bounds behavior were all flagged
from source first, then found to be already reasoned through in the docs.

**Audit boundary:** this is source-and-artifact review. It does NOT verify against live
chain state. Genesis UTXO value/covenant_id, txid existence, and deployed-script-hash match
are taken on the docs' word. An on-chain confirmation pass is the natural follow-up (see
Recommendation R4).

---

## Artifacts reviewed

| Artifact | Role | Verdict |
|---|---|---|
| `wallet.html` | web wallet (onboarding + HOME + action panels) | sound; write-side stubbed |
| `vendor/` (@scure/@noble) | vendored crypto, all v2.2.0 | real published libs, unmodified |
| `bitphoque_p2sh.js` | in-browser P2SH derivation (template splice) | mechanically correct, self-test reproduced |
| `bitphoque_sale_v3.sil` | the live covenant (KCC20, v3) | core invariants enforced |
| `bitphoque_genesis_establish.rs` | genesis covenant establishment | correct; stale node const |
| `bitphoque_compiled_real.json` | compiled contract + genesis ctor args | decoded; confirms no-minter genesis |
| `bitphoque_descriptor_v1.rs` | parcel descriptor codec (send/receive keystone) | cleanest component; round-trip + checksum reproduced |

---

## Verified clean (independently confirmed)

**Crypto stack.** The vendored dependency tree is the real published @scure/@noble stack,
all at matching 2.2.0 (noble-curves, noble-hashes, scure-base, scure-bip32, scure-bip39). No
homemade primitives in the dependency tree. The only hand-rolled crypto is the Kaspa bech32
codec in `wallet.html`, which is appropriate (no published lib does it) and is verified
against a real keygen vector plus an external KasWare cross-check (per docs).

**Encrypted persistence.** AES-GCM under PBKDF2-SHA256, 250k iterations, ciphertext-only in
localStorage, passphrase never stored, wrong passphrase fails the GCM auth tag (loud, not
silent-garbage). Sound as written.

**P2SH derivation (template splice).** Reproduced independently: template is exactly 2793
bytes; blake2b-256 of the spliced redeem script yields `f8a010c4…cdaee`, matching the claimed
on-chain value. The four operand slots land exactly where declared (owner @2/32B, amount
@37/8B-LE, forSale @48/1B, price @50/8B-LE), each framed by a correct push opcode and followed
immediately by an opcode, so large values fill their slot without colliding into neighboring
script bytes. Splice idempotency holds. The self-test is real, not a hardcoded pass.

**Covenant core invariants (`bitphoque_sale_v3.sil`).** All four advertised invariants are
genuinely enforced by the contract, not merely asserted in UI copy:
- *Conservation* — `checkAmounts` requires `sum(inputs.amount) == sum(outputs.amount)`; split,
  merge, and partial-list cannot create or destroy phoques.
- *Listing/claim separation* — `checkSaleState` blocks any for-sale output when any input is
  for-sale (correct anti-double-claim rule).
- *Claim authorization* — `checkAuth` enforces payment ≥ price to the seller's P2PK lock, no
  owner signature required.
- *F1 double-claim fix is real* — `require(w > lastSaleWitness)` forces strictly-increasing
  payment-output indices, so a buyer cannot point two for-sale inputs at one payment output to
  satisfy two listings with one payment.

**Descriptor codec (`bitphoque_descriptor_v1.rs`).** Reproduced independently: 118-byte layout
sums correctly, base58check round-trips, checksum guard fires on a single flipped character,
`forSale` byte and exact length are validated on decode. The transport/derivation decoupling is
correct — fixed-width u64-LE on the wire, CScriptNum only at derivation, with `derive_p2sh`
reusing the canonical `compile_contract` path rather than re-specifying the encoding (avoids the
classic two-implementations-drift bug). The covenant_id is the integrity tie: a wrong-token
descriptor is refused before any P2SH is derived against wrong constants.

---

## Findings

Severity key: **High** = can cause loss/lock of funds or break a core guarantee;
**Medium** = correctness/robustness gap with a plausible path to harm; **Low** = hygiene,
coverage, or latent/forward-looking risk.

### Resolved during reconciliation (raised from source, answered by docs/artifacts)

**F-C (was High) — Fixed-supply guarantee is off-covenant. RESOLVED.**
From source: the covenant's conservation and minter-propagation checks are both gated
`if(!isMinter)`, so the contract does not cap supply; it conserves non-minter value and forbids
minter creation. The guarantee therefore depends entirely on genesis. Decoding
`bitphoque_compiled_real.json` confirms `genesisIsMinter = false`, `genesisAmount = 42069`,
`idType = 0x00`. No minter UTXO exists in the design; the `isMinter` exemption is unreachable on
the live token. Supply is fixed because conservation holds on every reachable spend and the only
entry point (genesis) is a conserving non-minter. Docs state this precisely (no minter deployed,
every UTXO genesis'd `isMinter=false`). **The guarantee holds, via no-minter genesis rather than
a covenant cap.**

**F-B (was Medium/High) — Numeric operand width / high-price fund-lock. RESOLVED.**
From source: amount/price are pushed as fixed-width 8-byte LE in the P2SH template and compared
numerically (`int`) in the covenant. Initial concern: if the toolchain enforced a 4-byte
CScriptNum operand limit (~2³¹ ≈ 21.5 KAS on price), a high-priced listing could become
unspendable. Docs resolve this: ctor `int` encoding is pinned to minimal CScriptNum at
*derivation* (verified vs `serialize_i64`, `data_stack.rs:153`) and the script-number width is
i64, not 4-byte. The supply integer is explicitly overflow-checked against the i64 ceiling
(42,069 × 10⁸ ≈ 4.2×10¹⁵ « 9.2×10¹⁸). The fund-lock scenario is not reachable in practice.

**F-D (was Medium) — Attacker-supplied witness indexes `tx.outputs[]`. RESOLVED.**
From source: `checkAuth` does `int(witnesses[i])` on an attacker-supplied witness and uses it to
index `tx.outputs[...]`; strict-increasing constrains ordering but not range. Question: what
happens on an out-of-range index? Docs confirm empirically (the 2×2-claim build) that an
out-of-bounds witness read fails closed with `OutOfBoundsSubstring(...)` — the engine rejects,
it does not read garbage. Bounds behavior is fail-closed.

### Open findings (net-new or acknowledged-but-unclosed)

**F-1 (Medium) — Write side / node seam is unbuilt; `wallet.html` may be a pre-wiring snapshot.**
In the audited `wallet.html`, `receive`, `import`, and all write panels are NODE-SEAM stubs that
print the CLI-equivalent call and broadcast nothing; `refreshBalance` derives addresses but the
prototype comments describe the node as unreachable. The descriptor codec is ready and the wallet
has not yet wired it into a real `receive`. The latest status session reports in-browser P2SH
derivation + wRPC transport as proven with `refreshBalance` wiring as the next step — so the
audited file may trail the working tree by one step. **Action: confirm the audited `wallet.html`
is current.** Until the write side is built, the wallet is an onboarding + read shell, not a
spending wallet. (Consistent with docs; publish is correctly deferred.)

**F-2 (Low) — Derivation proven only against a test fixture, never the genesis owner.**
This is the one finding the docs do not already cover. Both the P2SH self-test and the descriptor
self-test assert against the buyer1 *test* parcel (`8c02c2…`); neither has ever asserted against
the real genesis owner (`dc2273…`). The derivation mechanism is identical, so severity is low,
but "proven for a test parcel" is not "proven end-to-end for the actual token holding." Cheap to
close (see R1).

**F-3 (Low, forward-looking) — Latent minter branch in reusable contract source.**
The covenant source still contains an unbounded mint path gated only on `isMinter`. Unreachable
for this token (no-minter genesis), but if the source is reused to launch a second token, or
recompiled with `genesisIsMinter = true`, the supply guarantee silently evaporates — the safety
property lives in the genesis params, not the covenant code. Docs note `kcc20-minter.sil` is
deliberately unused. **Recommend an in-source comment** that the minter branch must never be
genesis-enabled for a fixed-supply token.

**F-4 (Informational) — Stale node constant in genesis tool.**
`bitphoque_genesis_establish.rs` hardcodes `NODE = "192.168.0.206:16210"`. Per infra notes, .206
is permanent mainnet-only and the TN10 node has moved to Big Daddy (`127.0.0.1:16210`). This is a
stale constant covered by the project's intentional "fix-on-use, not bulk" policy — it will
fail-closed (connection error) on next run before it can do harm. Not a contamination bug; noted
for completeness. **Confirm/repoint before any genesis `--go`.**

---

## Reconciliation summary

The independent reconstruction found **no contradiction between claimed and actual behavior on
any core invariant.** Every High/Medium concern raised from source was either already reasoned
through in the project docs (F-B, F-C, F-D) or is a known, policy-covered item (F-4). The docs'
framing was in several cases more precise than the initial audit flag (notably the supply story,
correctly characterized as a genesis decision rather than a covenant guarantee).

The audit's net-new contribution is small and specific: **F-2** (test coverage never exercises
the real genesis parcel) is the only gap the documentation does not already address.

---

## Recommendations (priority order)

**R1 — Close F-2 with one test.** Run the P2SH and descriptor self-tests once against the real
genesis parcel (owner `dc2273…`, amount 42069, forSale=false), asserting against the live genesis
scriptPubKey on-chain. Converts "derivation proven for a test parcel" into "derivation proven for
the actual token," and closes the only doc-uncovered gap.

**R2 — Confirm `wallet.html` currency (F-1).** Verify the audited file matches the working tree;
if the read-side wiring (`refreshBalance` → real wRPC) has landed, re-audit that path. Keep
publish gated on the write side per existing plan.

**R3 — Annotate the minter branch (F-3).** Add a source comment in the covenant that the
`isMinter` path must never be genesis-enabled for a fixed-supply deployment. Protects against
future reuse of the contract as a KCC20 template.

**R4 — On-chain confirmation pass (audit boundary).** This audit reviewed code, not chain state.
A complete audit closes by confirming, against the live node: the genesis UTXO's value
(== 42,069 phoques' worth) and covenant_id; that the key asserted txids exist and mined; and that
the deployed script hash matches the template. Recommended before any mainnet framing.

**R5 — Repoint the genesis node constant (F-4).** Update or confirm the `NODE` const before any
genesis broadcast, even though fix-on-use will catch it.

---

## Overall assessment

BitPhoque is in substantially better shape than typical testnet-stage covenant projects. The
crypto is real and vendored; conservation and sale invariants are genuinely covenant-enforced;
the v3 double-claim fix is real; fixed supply holds via no-minter genesis; the descriptor codec
is clean and properly decoupled. The design discipline held up under an adversarial,
history-blind read — the most useful result an audit of this kind can produce. Remaining work is
the write-side wiring (known, planned) plus the small coverage and hygiene items above. No
fund-loss or supply-break vector was found in the reviewed source.
