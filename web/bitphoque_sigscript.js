/* ============================================================================
   BitPhoque — in-browser sigscript builder (covenant ABI + redeem push)
   ----------------------------------------------------------------------------
   Produces the signature_script bytes for a BitPhoque covenant transaction,
   matching what silverscript-lang's build_sig_script_for_covenant_decl emits.

   STRUCTURE (proven via CLI diagnostic hex dump):
     1. State[] arg — TRANSPOSED: each State field pushed separately as a
        concatenated fixed-width blob across all states.
        Field order (from contract AST):
          ownerIdentifier  byte[32]  → 32×N bytes
          identifierType   byte      → 1×N bytes
          amount           int       → 8×N bytes (u64 LE each)
          isMinter         bool      → 1×N bytes
          forSale          bool      → 1×N bytes
          price            int       → 8×N bytes (u64 LE each)
     2. sig[] arg — concatenated 65-byte sigs (schnorr 64B + hashtype 1B)
     3. byte[] witnesses arg — concatenated raw bytes
     4. Function selector — CScriptNum: 0 = OP_0 (__leader_transfer),
                                        1 = OP_1 (__delegate_transfer)
     5. Redeem script push — OP_PUSHDATA2 + u16le(len) + full redeem script

   DEPENDS ON: bitphoque_p2sh.js (deriveRedeemScript for the redeem push).
   ============================================================================ */

// --- push-opcode helpers (standard Bitcoin script encoding) -------------------
// These match ScriptBuilder::add_data in rusty-kaspa.

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
// 0 → OP_0 (0x00); 1–16 → OP_1..OP_16 (0x51..0x60).
function pushInt(n) {
  if (n === 0) return new Uint8Array([0x00]);
  if (n >= 1 && n <= 16) return new Uint8Array([0x50 + n]);
  // For larger values, encode as minimal CScriptNum + pushData.
  // BitPhoque only uses selectors 0 and 1, so this path is not exercised.
  throw new Error('pushInt > 16 not implemented (not needed for BitPhoque)');
}

// --- byte helpers ------------------------------------------------------------
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

// --- State struct for the ABI ------------------------------------------------
// Each state: { owner: Uint8Array(32), identifierType: 0, amount: BigInt,
//               isMinter: false, forSale: bool, price: BigInt }

// Build the transposed State[] push sequence.
// Returns an array of Uint8Array fragments (each is a pushData call).
function pushStates(states) {
  const n = states.length;
  if (n === 0) throw new Error('at least one new state required');

  // ownerIdentifier: 32 bytes per state
  const owners = new Uint8Array(32 * n);
  for (let i = 0; i < n; i++) owners.set(states[i].owner, i * 32);

  // identifierType: 1 byte per state
  const idTypes = new Uint8Array(n);
  for (let i = 0; i < n; i++) idTypes[i] = states[i].identifierType || 0;

  // amount: 8 bytes LE per state
  const amounts = new Uint8Array(8 * n);
  for (let i = 0; i < n; i++) amounts.set(u64le(states[i].amount), i * 8);

  // isMinter: 1 byte per state (always false for BitPhoque)
  const minters = new Uint8Array(n);
  for (let i = 0; i < n; i++) minters[i] = states[i].isMinter ? 1 : 0;

  // forSale: 1 byte per state
  const sales = new Uint8Array(n);
  for (let i = 0; i < n; i++) sales[i] = states[i].forSale ? 1 : 0;

  // price: 8 bytes LE per state
  const prices = new Uint8Array(8 * n);
  for (let i = 0; i < n; i++) prices.set(u64le(states[i].price || 0n), i * 8);

  return [
    pushData(owners),
    pushData(idTypes),
    pushData(amounts),
    pushData(minters),
    pushData(sales),
    pushData(prices),
  ];
}

// --- build the full signature_script -----------------------------------------
//
// states: array of State objects (the newStates for the covenant outputs)
// sigs: array of Uint8Array, each 65 bytes (schnorr sig 64 + hashtype 1)
// witnesses: Uint8Array (raw witness bytes, e.g. [0] for owner-signed)
// redeemScript: Uint8Array (the 2793-byte compiled covenant script)
// isLeader: bool (true for leader, false for delegate)
//
// For DELEGATE (merge input 1): args are empty, only selector + redeem push.
//
function buildSigScript(states, sigs, witnesses, redeemScript, isLeader) {
  const parts = [];

  if (isLeader) {
    // 1. transposed State[] fields
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

  // 5. redeem script push (2793 bytes → OP_PUSHDATA2)
  parts.push(pushData(redeemScript));

  return concatBytes.apply(null, parts);
}

// --- self-test against the CLI dry-run diagnostic ----------------------------
//
// The CLI split dry-run (buyer2, 5000→1000+4000) produced:
//   abi_sigscript (177 bytes): 405d338aa5...010000
//   sighash: aef9823c78ef4bae0c91fa4e09489f93d9ebb4f61fbf103258bf9147d70df13c
//
// We verify the ABI portion (before the redeem push) matches byte-for-byte.
//
function selfTest() {
  const buyer2 = hexToBytes('5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161');
  // The sig from the dry-run (64-byte schnorr + 01 hashtype)
  const sig = hexToBytes('380593f1e596786deeeb2d18994a43d313706ba50919902f36e2a28cf4911bbfaab70f3d3b3c03c57a2238c832f7c7b732b0a9953bbd60da8322fea06934e0d801');

  const states = [
    { owner: buyer2, identifierType: 0, amount: 1000n, isMinter: false, forSale: false, price: 0n },
    { owner: buyer2, identifierType: 0, amount: 4000n, isMinter: false, forSale: false, price: 0n },
  ];

  // Build ABI-only (no redeem push) by constructing the parts manually
  const parts = [];
  for (const p of pushStates(states)) parts.push(p);
  parts.push(pushData(sig));                         // sigs[]
  parts.push(pushData(new Uint8Array([0x00])));      // witnesses
  parts.push(pushInt(0));                            // selector

  const abiBytes = concatBytes.apply(null, parts);
  const abiHex = bytesToHex(abiBytes);

  const expectedHex = '405d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf1615d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf16102000010e803000000000000a00f000000000000020000020000100000000000000000000000000000000041380593f1e596786deeeb2d18994a43d313706ba50919902f36e2a28cf4911bbfaab70f3d3b3c03c57a2238c832f7c7b732b0a9953bbd60da8322fea06934e0d801010000';

  const ok = abiHex === expectedHex;
  console.log('[sigscript selfTest] abi-bytes (' + abiBytes.length + 'B)',
    ok ? 'PASS' : 'FAIL',
    ok ? '' : '\n  got: ' + abiHex + '\n  exp: ' + expectedHex);
  return ok;
}

// --- attach to wallet namespace if present -----------------------------------
if (typeof window !== 'undefined' && window.KASPA) {
  window.KASPA.buildSigScript = buildSigScript;
  window.KASPA.sigscriptSelfTest = selfTest;
}

export { buildSigScript, pushData, pushInt, concatBytes, hexToBytes, bytesToHex, u64le, selfTest };
