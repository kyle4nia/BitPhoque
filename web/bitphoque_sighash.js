/* ============================================================================
   BitPhoque — in-browser Kaspa sighash (Toccata v1, SIG_HASH_ALL)
   ----------------------------------------------------------------------------
   Reimplements `calc_schnorr_signature_hash` from rusty-kaspa in pure JS so
   the browser can sign covenant transactions without WASM or a backend.

   The algorithm is a keyed blake2b-256 hash over a defined sequence of tx
   fields, matching rusty-kaspa consensus/core/src/hashing/sighash.rs exactly.
   Validated against the Rust unit-test vectors (selfTest below).

   SCOPE: Toccata (version >= 1) transactions with SIG_HASH_ALL only.
   BitPhoque always uses these; v0 and other hash types are out of scope.

   DEPENDS ON: vendored @noble/hashes blake2b (already in web/vendor/).
   ============================================================================ */
import { blake2b } from './vendor/noble-hashes/blake2.js';

// --- constants ---------------------------------------------------------------
// Domain separator: blake2b KEY (not personalization), matching
//   Params::new().hash_length(32).key(b"TransactionSigningHash")
// from kaspa-hashes/src/hashers.rs  (blake2b_hasher! macro).
const SIGNING_KEY = new TextEncoder().encode("TransactionSigningHash");
const ZERO_HASH = new Uint8Array(32);
const SIG_HASH_ALL = 0x01;

// --- byte helpers (all little-endian, matching Kaspa HasherExtensions) --------
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
function u16le(n) {
  return new Uint8Array([n & 0xff, (n >> 8) & 0xff]);
}
function u32le(n) {
  return new Uint8Array([n & 0xff, (n >> 8) & 0xff, (n >> 16) & 0xff, (n >> 24) & 0xff]);
}
function u64le(n) {
  const b = new Uint8Array(8);
  let v = BigInt(n);
  for (let i = 0; i < 8; i++) { b[i] = Number(v & 0xffn); v >>= 8n; }
  return b;
}

// --- keyed blake2b-256 hasher (streaming) ------------------------------------
function newSigningHasher() {
  return blake2b.create({ key: SIGNING_KEY, dkLen: 32 });
}

// --- sub-hashes (cached per-tx, recomputed here — no quadratic concern in JS
//     since we only sign 1–3 inputs per tx) -----------------------------------

// write_var_bytes: u64le(len) || bytes  (HasherExtensions::write_var_bytes)
function varBytes(h, bytes) {
  h.update(u64le(bytes.length));
  if (bytes.length > 0) h.update(bytes);
}

// hash_script_public_key: u16le(version) || var_bytes(script)
function hashScriptPubKey(h, version, script) {
  h.update(u16le(version));
  varBytes(h, script);
}

// previous_outputs_hash: blake2b of all input (txid || u32le index)
function previousOutputsHash(tx) {
  const h = newSigningHasher();
  for (const inp of tx.inputs) {
    h.update(inp.prevTxId);      // Uint8Array, 32 bytes
    h.update(u32le(inp.prevIndex));
  }
  return h.digest();
}

// sequences_hash: blake2b of all input u64le(sequence)
function sequencesHash(tx) {
  const h = newSigningHasher();
  for (const inp of tx.inputs) {
    h.update(u64le(inp.sequence));
  }
  return h.digest();
}

// hash_output (v1+): u64le(value) || scriptPubKey || bool(hasCovenant)
//   if covenant: u16le(authorizingInput) || covenantId(32)
function hashOutput(h, out, version) {
  h.update(u64le(out.value));
  hashScriptPubKey(h, out.scriptPubKeyVersion, out.scriptPubKey);
  if (version >= 1) {
    const hasCov = !!out.covenant;
    h.update(new Uint8Array([hasCov ? 1 : 0]));
    if (hasCov) {
      h.update(u16le(out.covenant.authorizingInput));
      h.update(out.covenant.covenantId);   // 32 bytes
    }
  }
}

// outputs_hash: blake2b of all outputs (SIG_HASH_ALL path)
function outputsHash(tx) {
  const h = newSigningHasher();
  for (const out of tx.outputs) {
    hashOutput(h, out, tx.version);
  }
  return h.digest();
}

// payload_hash: ZERO_HASH for native subnetwork + empty payload
function payloadHash(tx) {
  const isNative = tx.subnetworkId.every(b => b === 0);
  if (isNative && tx.payload.length === 0) return ZERO_HASH;
  const h = newSigningHasher();
  varBytes(h, tx.payload);
  return h.digest();
}

// --- main sighash (Toccata v1, SIG_HASH_ALL) ---------------------------------
//
// tx = {
//   version: 1,
//   inputs: [{ prevTxId: Uint8Array(32), prevIndex: number, sequence: BigInt|number }],
//   outputs: [{ value: BigInt, scriptPubKeyVersion: number, scriptPubKey: Uint8Array,
//               covenant: null | { authorizingInput: number, covenantId: Uint8Array(32) } }],
//   lockTime: BigInt,
//   subnetworkId: Uint8Array(20),
//   gas: BigInt,
//   payload: Uint8Array,
// }
//
// utxoEntries = [{ amount: BigInt, scriptPubKeyVersion: number, scriptPubKey: Uint8Array }]
//   one per input, same order.
//
// Returns Uint8Array(32) — the 32-byte hash to be schnorr-signed.
//
function calcSchnorrSignatureHash(tx, inputIndex, utxoEntries, hashType) {
  if (tx.version < 1) throw new Error('v0 sighash not implemented (BitPhoque uses Toccata v1)');
  if (hashType !== SIG_HASH_ALL) throw new Error('only SIG_HASH_ALL implemented');

  const inp = tx.inputs[inputIndex];
  const utxo = utxoEntries[inputIndex];
  const h = newSigningHasher();

  // 1. tx version
  h.update(u16le(tx.version));

  // 2. previous_outputs_hash (all inputs' outpoints)
  h.update(previousOutputsHash(tx));

  // 3. sequences_hash (all inputs' sequences)
  h.update(sequencesHash(tx));

  // v1+: NO sig_op_counts_hash

  // 4. this input's outpoint
  h.update(inp.prevTxId);
  h.update(u32le(inp.prevIndex));

  // 5. this input's UTXO scriptPubKey
  hashScriptPubKey(h, utxo.scriptPubKeyVersion, utxo.scriptPubKey);

  // 6. this input's UTXO amount
  h.update(u64le(utxo.amount));

  // 7. this input's sequence
  h.update(u64le(inp.sequence));

  // v1+: NO per-input sig_op_count

  // 8. outputs_hash (all outputs)
  h.update(outputsHash(tx));

  // 9. lock_time
  h.update(u64le(tx.lockTime));

  // 10. subnetwork_id (20 bytes)
  h.update(tx.subnetworkId);

  // 11. gas
  h.update(u64le(tx.gas));

  // 12. payload_hash
  h.update(payloadHash(tx));

  // 13. hash_type
  h.update(new Uint8Array([hashType]));

  return h.digest();
}

// --- self-test against rusty-kaspa unit-test vectors -------------------------
//
// Two gates:
//   (a) bare keyed-blake2b sanity (empty input, from hashers.rs test_vectors)
//   (b) full v1 sighash (native-v1-all-0, from sighash.rs test_signature_hash)
//
function selfTest() {
  let ok = true;

  // (a) bare blake2b keyed hash: blake2b(b"", key=b"TransactionSigningHash", dkLen=32)
  //     expected from hashers.rs test_vectors, TransactionSigningHash row, input &[]
  const bareExpect = '34c75037ad62740d4b3228f88f844f7901c07bfacd55a045be518eabc15e52ce';
  const bareGot = bytesToHex(newSigningHasher().digest());
  const bareOk = bareGot === bareExpect;
  console.log('[sighash selfTest] blake2b-key', bareOk ? 'PASS' : 'FAIL', bareOk ? '' : 'got ' + bareGot);
  ok = ok && bareOk;

  // (b) full v1 sighash: native-v1-all-0 from sighash.rs
  const prevTxId = hexToBytes('880eb9819a31821d9d2399e2f35e2433b72637e393d71ecc9b8d0250f49153c3');
  const spk1 = hexToBytes('208325613d2eeaf7176ac6c670b13c0043156c427438ed72d74b7800862ad884e8ac');
  const spk2 = hexToBytes('20fcef4c106cf11135bbd70f02a726a92162d2fb8b22f0469126f800862ad884e8ac');

  const tx = {
    version: 1,
    inputs: [
      { prevTxId, prevIndex: 0, sequence: 0n },
      { prevTxId, prevIndex: 1, sequence: 1n },
      { prevTxId, prevIndex: 2, sequence: 2n },
    ],
    outputs: [
      { value: 300n, scriptPubKeyVersion: 0, scriptPubKey: spk2, covenant: null },
      { value: 300n, scriptPubKeyVersion: 0, scriptPubKey: spk1, covenant: null },
    ],
    lockTime: 1615462089000n,
    subnetworkId: new Uint8Array(20),
    gas: 0n,
    payload: new Uint8Array(0),
  };

  const utxoEntries = [
    { amount: 100n, scriptPubKeyVersion: 0, scriptPubKey: spk1 },
    { amount: 200n, scriptPubKeyVersion: 0, scriptPubKey: spk2 },
    { amount: 300n, scriptPubKeyVersion: 0, scriptPubKey: spk2 },
  ];

  const v1Expect = '5b2657524be672e019897646b56da3d192b453d78ae5e6e5c07f029a69f5f075';
  const v1Got = bytesToHex(calcSchnorrSignatureHash(tx, 0, utxoEntries, SIG_HASH_ALL));
  const v1Ok = v1Got === v1Expect;
  console.log('[sighash selfTest] v1-all-0', v1Ok ? 'PASS' : 'FAIL', v1Ok ? '' : 'got ' + v1Got);
  ok = ok && v1Ok;

  return ok;
}

// --- attach to wallet namespace if present -----------------------------------
if (typeof window !== 'undefined' && window.KASPA) {
  window.KASPA.calcSchnorrSignatureHash = calcSchnorrSignatureHash;
  window.KASPA.sighashSelfTest = selfTest;
  window.KASPA.SIG_HASH_ALL = SIG_HASH_ALL;
}

export {
  calcSchnorrSignatureHash, selfTest, SIG_HASH_ALL,
  hexToBytes, bytesToHex, u16le, u32le, u64le,
  newSigningHasher, previousOutputsHash, sequencesHash, outputsHash, payloadHash,
};
