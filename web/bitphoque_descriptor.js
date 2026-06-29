// bitphoque_descriptor.js
// ---------------------------------------------------------------------------
// PARCEL DESCRIPTOR CODEC (v1) — browser/ESM mirror of bitphoque_descriptor_v1.rs.
// The receive keystone: decode the off-chain "receipt" a sender hands you so a
// fresh wallet can FIND and VERIFY a parcel whose P2SH is amount-derived.
//
// PROVEN byte-identical to the Rust codec (2026-06-26): decodes the canonical
// --selftest string back to the exact fields AND re-encodes to the identical
// 166-char base58check string. Checksum scheme (double-sha256) matches Rust's
// bs58 .with_check(). Confirmed against the live 4b0c8187..:0 parcel fixture.
//
// FORMAT v1 (118-byte payload, then base58check-wrapped):
//   off  field                       bytes  enc
//   0    version                     1      u8
//   1    covenant_id                 32     raw
//   33   owner_pubkey (x-only)       32     raw
//   65   amount                      8      u64 LE
//   73   forSale                     1      0/1
//   74   price                       8      u64 LE
//   82   txid                        32     raw
//   114  output_index                4      u32 LE   = 118 B
//
// Transport carries LOGICAL integer values (fixed-width). CScriptNum lives ONLY
// in P2SH derivation (bitphoque_p2sh.js) — transport and derivation are
// deliberately decoupled. idType/isMinter/caps are protocol CONSTANTS, derived
// not transported. covenant_id is the integrity tie: a descriptor whose id we
// don't recognize is REFUSED rather than derived against the wrong constants.
//
// amount/price are returned as BigInt (exact past 2^53). Pass them as BigInt,
// number, or decimal string to encode.
// ---------------------------------------------------------------------------

import { base58check } from './vendor/scure-base/index.js';
import { sha256 } from './vendor/noble-hashes/sha2.js';
import { COVENANT_ID_HEX } from './bitphoque_config.js';

// base58check(sha256) => checksum = sha256(sha256(payload))[0..4], byte-for-byte
// identical to Rust bs58 .with_check(). Verified by round-trip against the chain.
const b58check = base58check(sha256);

export const DESC_LEN = 118;
export const DESC_VERSION = 1;

// v3 covenant_id — the integrity tie. Descriptors for any other token are refused.
// Covenant ID imported from bitphoque_config.js
export const V3_COVENANT_ID = COVENANT_ID_HEX;

// --- hex helpers ---
const toHex = (u8) =>
  Array.from(u8, (b) => b.toString(16).padStart(2, '0')).join('');

function fromHex(h, expectBytes, what) {
  if (typeof h !== 'string' || h.length % 2 !== 0)
    throw new Error(`${what}: expected even-length hex string`);
  const a = new Uint8Array(h.length / 2);
  for (let i = 0; i < a.length; i++) {
    const byte = parseInt(h.substr(i * 2, 2), 16);
    if (Number.isNaN(byte)) throw new Error(`${what}: bad hex`);
    a[i] = byte;
  }
  if (expectBytes != null && a.length !== expectBytes)
    throw new Error(`${what}: expected ${expectBytes} bytes, got ${a.length}`);
  return a;
}

// --- little-endian integer codecs (BigInt for u64 to stay exact) ---
function putU64le(arr, off, val) {
  let v = BigInt(val);
  if (v < 0n || v > 0xffffffffffffffffn)
    throw new Error('u64 out of range');
  for (let i = 0; i < 8; i++) { arr[off + i] = Number(v & 0xffn); v >>= 8n; }
}
function getU64le(arr, off) {
  let v = 0n;
  for (let i = 7; i >= 0; i--) v = (v << 8n) | BigInt(arr[off + i]);
  return v;
}
function putU32le(arr, off, val) {
  let v = (typeof val === 'bigint' ? Number(val) : val) >>> 0;
  for (let i = 0; i < 4; i++) { arr[off + i] = v & 0xff; v >>>= 8; }
}
function getU32le(arr, off) {
  return (
    (arr[off] | (arr[off + 1] << 8) | (arr[off + 2] << 16) | (arr[off + 3] << 24)) >>> 0
  );
}

// ---------------------------------------------------------------------------
// encode: fields -> 118-byte payload -> base58check string.
// d = { covenantId, ownerPubkey, amount, forSale, price, txid, outputIndex }
//      (version defaults to DESC_VERSION). hex fields are 64-char (32 byte).
// ---------------------------------------------------------------------------
export function encodeDescriptor(d) {
  const p = new Uint8Array(DESC_LEN);
  p[0] = d.version ?? DESC_VERSION;
  p.set(fromHex(d.covenantId, 32, 'covenant_id'), 1);
  p.set(fromHex(d.ownerPubkey, 32, 'owner_pubkey'), 33);
  putU64le(p, 65, d.amount);
  p[73] = d.forSale ? 1 : 0;
  putU64le(p, 74, d.price);
  p.set(fromHex(d.txid, 32, 'txid'), 82);
  putU32le(p, 114, d.outputIndex);
  return b58check.encode(p);
}

// ---------------------------------------------------------------------------
// decode: base58check string -> validate checksum + length + guards -> fields.
// Fails LOUD on: bad checksum (mis-typed receipt), wrong length, forSale not in
// {0,1}, or an unrecognized covenant_id (descriptor not for this token).
// ---------------------------------------------------------------------------
export function decodeDescriptor(s) {
  let p;
  try {
    p = b58check.decode(s);
  } catch (e) {
    throw new Error(
      'base58check decode failed (bad checksum or alphabet): ' + e.message
    );
  }
  if (p.length !== DESC_LEN)
    throw new Error(`descriptor payload is ${p.length} bytes, expected ${DESC_LEN}`);

  const fs = p[73];
  if (fs !== 0 && fs !== 1)
    throw new Error(`forSale byte must be 0 or 1, got ${fs}`);

  const d = {
    version: p[0],
    covenantId: toHex(p.slice(1, 33)),
    ownerPubkey: toHex(p.slice(33, 65)),
    amount: getU64le(p, 65),
    forSale: fs === 1,
    price: getU64le(p, 74),
    txid: toHex(p.slice(82, 114)),
    outputIndex: getU32le(p, 114),
  };

  if (d.covenantId !== V3_COVENANT_ID)
    throw new Error(
      `unrecognized covenant_id ${d.covenantId} — descriptor not for this token, refusing`
    );

  return d;
}
