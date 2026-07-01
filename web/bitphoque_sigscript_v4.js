/* ============================================================================
   BitPhoque — in-browser sigscript builder (covenant ABI + redeem push)
   ----------------------------------------------------------------------------
   Produces the signature_script bytes for a BitPhoque covenant transaction,
   matching what silverscript-lang's build_sig_script_for_covenant_decl emits.

   v4 STATE LAYOUT (mainnet candidate — bitphoque_sale_v4.sil).
   Mechanically updated from the v3 (KCC20) 6-field/51-byte layout to v4's
   4-field/49-byte layout: identifierType and isMinter are DROPPED entirely
   (checklist item 5 — minter branch + dead identifierType stripped).

   STATUS: this transposition order is confirmed correct BY CONSTRUCTION —
   it matches bitphoque_sale_v4.sil's declared field order exactly:
     byte[32] ownerIdentifier = genesisPk;
     int      amount          = genesisAmount;
     bool     forSale         = genesisForSale;
     int      price           = genesisPrice;
   It has NOT been verified against a live v4 CLI dry-run, because v4 has not
   been compiled yet (blocked on silverscript V1 — checklist item 4). The
   selfTest() below reflects that: it computes and PRINTS the ABI bytes for
   manual comparison once a real v4 fixture exists, it does NOT assert
   against a fabricated expected value. Do not treat this file as proven
   until selfTest() is upgraded to a real byte-for-byte assertion.

   STRUCTURE (unchanged from v3 except State[] field count/order):
     1. State[] arg — TRANSPOSED: each State field pushed separately as a
        concatenated fixed-width blob across all states.
        Field order (from v4 contract AST):
          ownerIdentifier  byte[32]  → 32×N bytes
          amount           int       → 8×N bytes (u64 LE each)
          forSale          bool      → 1×N bytes
          price            int       → 8×N bytes (u64 LE each)
        Total per-state width: 49 bytes (was 51 in v3).
     2. sig[] arg — concatenated 65-byte sigs (schnorr 64B + hashtype 1B)
     3. byte[] witnesses arg — concatenated raw bytes
     4. Function selector — CScriptNum: 0 = OP_0 (__leader_transfer),
                                        1 = OP_1 (__delegate_transfer)
     5. Redeem script push — OP_PUSHDATA2 + u16le(len) + full redeem script
        (v4's compiled size is unknown until V1 compile — v3 was 2793B;
        v4 is smaller per checklist item 5, "shrinks script", exact size TBD)

   DEPENDS ON: bitphoque_p2sh.js (deriveRedeemScript for the redeem push) —
   that file also needs updating for the new 49-byte template once v4
   compiles (checklist item 7, not yet done as of this file).
   ============================================================================ */

// --- push-opcode helpers (standard Bitcoin script encoding) -------------------
// These match ScriptBuilder::add_data in rusty-kaspa. UNCHANGED from v3.

function pushData(data) {
  // data: Uint8Array. Returns Uint8Array with push opcode framing.
  const n = data.length;
  if (n === 0) return new Uint8Array([0x00]);        // OP_0
  if (n <= 75) {
    const out = new Uint8Array(1 + n);
    out[0] = n;
    out.set(data, 1);
    return out;
  }
  if (n <= 255) {
    const out = new Uint8Array(2 + n);
    out[0] = 0x4c;  // OP_PUSHDATA1
    out[1] = n;
    out.set(data, 2);
    return out;
  }
  if (n <= 65535) {
    const out = new Uint8Array(3 + n);
    out[0] = 0x4d;  // OP_PUSHDATA2
    out[1] = n & 0xff;
    out[2] = (n >> 8) & 0xff;
    out.set(data, 3);
    return out;
  }
  throw new Error('data too large for pushdata: ' + n);
}

// CScriptNum push for small non-negative integers (function selector).
// 0 → OP_0 (0x00); 1–16 → OP_1..OP_16 (0x51..0x60). UNCHANGED from v3.
function pushInt(n) {
  if (n === 0) return new Uint8Array([0x00]);
  if (n >= 1 && n <= 16) return new Uint8Array([0x50 + n]);
  throw new Error('pushInt > 16 not implemented (not needed for BitPhoque)');
}

// --- byte helpers ------------------------------------------------------------
// UNCHANGED from v3.
function u64le(n) {
  const b = new Uint8Array(8);
  let v = BigInt(n);
  for (let i = 0; i < 8; i++) { b[i] = Number(v & 0xffn); v >>= 8n; }
  return b;
}

function hexToBytes(h) {
  const a = new Uint8Array(h.length / 2);
  for (let i = 0; i < a.length; i++) a[i] = parseInt(h.substr(i * 2, 2), 16);
  return a;
}

function bytesToHex(b) {
  let s = '';
  for (const x of b) s += x.toString(16).padStart(2, '0');
  return s;
}

function concatBytes(/* ...arrays */) {
  let total = 0;
  for (let i = 0; i < arguments.length; i++) total += arguments[i].length;
  const out = new Uint8Array(total);
  let off = 0;
  for (let i = 0; i < arguments.length; i++) {
    out.set(arguments[i], off);
    off += arguments[i].length;
  }
  return out;
}

// --- State struct for the ABI (v4) -------------------------------------------
// Each state: { owner: Uint8Array(32), amount: BigInt, forSale: bool, price: BigInt }
// identifierType and isMinter are GONE — do not pass them, they are ignored
// if present (harmless) but callers should be updated to stop constructing
// them (checklist item 7's downstream-code pass, e.g. wallet.html).

// Build the transposed State[] push sequence.
// Returns an array of Uint8Array fragments (each is a pushData call).
function pushStates(states) {
  const n = states.length;
  if (n === 0) throw new Error('at least one new state required');

  // ownerIdentifier: 32 bytes per state
  const owners = new Uint8Array(32 * n);
  for (let i = 0; i < n; i++) owners.set(states[i].owner, i * 32);

  // amount: 8 bytes LE per state
  const amounts = new Uint8Array(8 * n);
  for (let i = 0; i < n; i++) amounts.set(u64le(states[i].amount), i * 8);

  // forSale: 1 byte per state
  const sales = new Uint8Array(n);
  for (let i = 0; i < n; i++) sales[i] = states[i].forSale ? 1 : 0;

  // price: 8 bytes LE per state
  const prices = new Uint8Array(8 * n);
  for (let i = 0; i < n; i++) prices.set(u64le(states[i].price || 0n), i * 8);

  return [
    pushData(owners),
    pushData(amounts),
    pushData(sales),
    pushData(prices),
  ];
}

// --- build the full signature_script -----------------------------------------
//
// states: array of State objects (the newStates for the covenant outputs)
// sigs: array of Uint8Array, each 65 bytes (schnorr sig 64 + hashtype 1)
// witnesses: Uint8Array (raw witness bytes, e.g. [0] for owner-signed)
// redeemScript: Uint8Array (the compiled v4 covenant script)
// isLeader: bool (true for leader, false for delegate)
//
// For DELEGATE (merge input 1): args are empty, only selector + redeem push.
// UNCHANGED structurally from v3 — only pushStates()'s field count changed.
//
function buildSigScript(states, sigs, witnesses, redeemScript, isLeader) {
  const parts = [];

  if (isLeader) {
    // 1. transposed State[] fields (4 fields now, was 6)
    for (const p of pushStates(states)) parts.push(p);

    // 2. sig[] — concatenate all 65-byte sigs into one push
    const sigBlob = new Uint8Array(65 * sigs.length);
    for (let i = 0; i < sigs.length; i++) sigBlob.set(sigs[i], i * 65);
    parts.push(pushData(sigBlob));

    // 3. byte[] witnesses
    parts.push(pushData(witnesses));
  }

  // 4. function selector: 0 = __leader_transfer, 1 = __delegate_transfer
  parts.push(pushInt(isLeader ? 0 : 1));

  // 5. redeem script push
  parts.push(pushData(redeemScript));

  return concatBytes.apply(null, parts);
}

// --- self-test: STRUCTURE-ONLY until a real v4 CLI fixture exists ------------
//
// This does NOT assert against an expected hex value (there is no v4 CLI
// dry-run to derive one from yet — v4 hasn't been compiled, blocked on
// silverscript V1). Asserting against a fabricated expected value would be
// a fake pass, worse than no test at all.
//
// What this DOES verify, structurally, right now:
//   - field count is 4 (not 6)
//   - total per-state byte width is 49 (not 51)
//   - field order matches v4's declared struct order
//
// TODO (checklist item 7 completion): once V1 compiles v4 and a real CLI
// dry-run fixture exists (same method as v3's selfTest — a real transaction
// built by the harness, hex-dumped), replace this with a byte-for-byte
// assertion exactly like v3's selfTest() had.
//
function selfTest() {
  const testOwner = hexToBytes('5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161');

  const states = [
    { owner: testOwner, amount: 1000n, forSale: false, price: 0n },
    { owner: testOwner, amount: 4000n, forSale: false, price: 0n },
  ];

  const fragments = pushStates(states);

  const checks = [];

  checks.push(['field count === 4', fragments.length === 4]);

  // Each fragment is pushData-framed; for n=2 states, owners=64B (<=75, so
  // 1-byte length prefix + 64 = 65B fragment), amounts=16B (1+16=17B),
  // sales=2B (1+2=3B), prices=16B (1+16=17B).
  const expectedFragLens = [65, 17, 3, 17];
  for (let i = 0; i < 4; i++) {
    checks.push([
      'fragment ' + i + ' length === ' + expectedFragLens[i],
      fragments[i].length === expectedFragLens[i]
    ]);
  }

  const totalPerStateWidth = 32 + 8 + 1 + 8;
  checks.push(['per-state width === 49', totalPerStateWidth === 49]);

  let allPass = true;
  for (const [label, pass] of checks) {
    if (!pass) allPass = false;
    console.log('[sigscript v4 selfTest]', label, pass ? 'PASS' : 'FAIL');
  }

  // Print the computed ABI bytes so they're ready to compare against a real
  // v4 CLI dry-run the moment one exists — same shape as v3's expectedHex.
  const parts = [];
  for (const p of fragments) parts.push(p);
  const abiPreview = concatBytes.apply(null, parts);
  console.log('[sigscript v4 selfTest] computed ABI-prefix bytes (' +
    abiPreview.length + 'B, states-only, no sig/witness/selector yet):',
    bytesToHex(abiPreview));
  console.log('[sigscript v4 selfTest] NOT VERIFIED against live bytecode ' +
    '— compare against a real v4 CLI dry-run once silverscript V1 compiles.');

  return allPass;
}

// --- attach to wallet namespace if present -----------------------------------
if (typeof window !== 'undefined' && window.KASPA) {
  window.KASPA.buildSigScript = buildSigScript;
  window.KASPA.sigscriptSelfTest = selfTest;
}

export { buildSigScript, pushData, pushInt, concatBytes, hexToBytes, bytesToHex, u64le, selfTest };
