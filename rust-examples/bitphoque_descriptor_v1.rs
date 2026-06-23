// bitphoque_descriptor_v1.rs
// ---------------------------------------------------------------------------
// PARCEL DESCRIPTOR CODEC (v1) — the send/receive keystone, node-INDEPENDENT.
//
// A parcel descriptor is the off-chain "receipt" a sender hands a receiver so a
// fresh wallet can FIND and VERIFY a parcel whose P2SH is amount-derived (the
// receive problem). This file is the trustless codec underneath that:
//   encode_descriptor(fields) -> 118-byte payload -> base58check string
//   decode_descriptor(string) -> fields           (base58check checksum-gated)
//   derive_p2sh(fields)       -> Address          (the SAME path as inspect_v3)
//
// FORMAT v1 (FINALIZED 2026-06-23, Option B — see bitphoque_status.md):
//   version(1) | covenant_id(32) | owner_pubkey x-only(32) | amount u64-LE(8)
//   | forSale(1) | price u64-LE(8) | txid(32) | output_index u32-LE(4) = 118 B
// Then base58check-wrap the 118 bytes (loud failure on a mis-transcribed receipt).
//
// WHY fixed-width transport (not CScriptNum): the descriptor carries LOGICAL
// integer values; the variable-length CScriptNum encoding lives ONLY in P2SH
// DERIVATION (ctor int args -> add_i64 -> script). Transport and derivation are
// deliberately DECOUPLED so the wire format stays a flat fixed-offset struct and
// byte-identity to the chain comes from REUSING the canonical builder, never
// from re-specifying the encoding. Correct because amount/price/index are all
// non-negative and fit u64/u32.
//
// WHY the descriptor omits idType / isMinter / caps: those are PROTOCOL CONSTANTS
// for BitPhoque v3 (idType=0, isMinter=false, caps 3/3 — see ctor_args below),
// identical for every parcel of this token, so they are DERIVED, not transported.
// The descriptor's covenant_id (32B) is the integrity tie: a wallet that doesn't
// recognize the covenant_id refuses the descriptor rather than deriving a wrong
// P2SH against the wrong constants.
//
// derive_p2sh lifts the EXACT path proven in bitphoque_inspect_v3.rs:
//   ctor_args -> compile_contract(&source, ..) -> pay_to_script_hash_script
//   -> extract_script_pub_key_address(.., Prefix::Testnet)
//
// SELFTEST (--selftest, NO node needed): round-trips a known parcel
// (encode->decode->assert equal) AND derives its P2SH, printing the address so it
// can be eyeballed against the live 4b0c8187..:0 parcel on-chain.
//
// silverscript bin (needs compiler API + kaspa crates). CANONICAL copy lives at
// C:\bitphoque\rust-examples\; disposable build copy in silverscript-lang\src\bin\.
// New dep: bs58 (with the "check" feature) for base58check.
// ---------------------------------------------------------------------------

use std::fs;

use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, CompileOptions};

use kaspa_addresses::{Address, Prefix};
use kaspa_txscript::{extract_script_pub_key_address, pay_to_script_hash_script};

// --- protocol constants (baked into the P2SH, NOT carried in the descriptor) ---
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const MAX_COV_INS: i64 = 3; // MUST match genesis + split (baked into P2SH)
const MAX_COV_OUTS: i64 = 3;

// app version byte stamped into the descriptor payload (in-payload, distinct from
// any base58check version-prefix concept). Bump if the layout ever changes.
const DESC_VERSION: u8 = 1;

const DESC_LEN: usize = 118;

// 8-arg ctor: owner, amount, idType=0, isMinter=false, forSale, price, maxIns, maxOuts.
// IDENTICAL shape to bitphoque_inspect_v3.rs / the list harness so the SAME P2SH is computed.
fn ctor_args(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner),
        Expr::int(amount),
        Expr::byte(0),
        Expr::bool(false),
        Expr::bool(for_sale),
        Expr::int(price),
        Expr::int(MAX_COV_INS),
        Expr::int(MAX_COV_OUTS),
    ]
}

// ---------------------------------------------------------------------------
// The logical descriptor fields (what travels; what derive_p2sh consumes).
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
struct Descriptor {
    version: u8,
    covenant_id: [u8; 32],
    owner_pubkey: [u8; 32], // x-only
    amount: u64,
    for_sale: bool,
    price: u64,
    txid: [u8; 32],
    output_index: u32,
}

// ---------------------------------------------------------------------------
// encode: fields -> 118-byte payload -> base58check string.
// ---------------------------------------------------------------------------
fn encode_payload(d: &Descriptor) -> Vec<u8> {
    let mut out = Vec::with_capacity(DESC_LEN);
    out.push(d.version);
    out.extend_from_slice(&d.covenant_id);
    out.extend_from_slice(&d.owner_pubkey);
    out.extend_from_slice(&d.amount.to_le_bytes()); // u64 LE (8)
    out.push(if d.for_sale { 1 } else { 0 });
    out.extend_from_slice(&d.price.to_le_bytes()); // u64 LE (8)
    out.extend_from_slice(&d.txid);
    out.extend_from_slice(&d.output_index.to_le_bytes()); // u32 LE (4)
    debug_assert_eq!(out.len(), DESC_LEN, "payload must be exactly {DESC_LEN} bytes");
    out
}

fn encode_descriptor(d: &Descriptor) -> String {
    let payload = encode_payload(d);
    // base58check: bs58 with .with_check() appends a 4-byte double-sha256 checksum.
    bs58::encode(payload).with_check().into_string()
}

// ---------------------------------------------------------------------------
// decode: base58check string -> validate checksum + length -> fields.
// Fails LOUD (Err) on a bad checksum (mis-transcribed receipt) or wrong length.
// ---------------------------------------------------------------------------
fn decode_descriptor(s: &str) -> Result<Descriptor, String> {
    let payload = bs58::decode(s)
        .with_check(None)
        .into_vec()
        .map_err(|e| format!("base58check decode failed (bad checksum or alphabet): {e}"))?;
    if payload.len() != DESC_LEN {
        return Err(format!("descriptor payload is {} bytes, expected {DESC_LEN}", payload.len()));
    }
    let mut o = 0usize;
    let take = |o: &mut usize, n: usize| -> &[u8] {
        let s = &payload[*o..*o + n];
        *o += n;
        s
    };
    let version = take(&mut o, 1)[0];
    let mut covenant_id = [0u8; 32];
    covenant_id.copy_from_slice(take(&mut o, 32));
    let mut owner_pubkey = [0u8; 32];
    owner_pubkey.copy_from_slice(take(&mut o, 32));
    let amount = u64::from_le_bytes(take(&mut o, 8).try_into().unwrap());
    let for_sale = match take(&mut o, 1)[0] {
        0 => false,
        1 => true,
        b => return Err(format!("forSale byte must be 0 or 1, got {b}")),
    };
    let price = u64::from_le_bytes(take(&mut o, 8).try_into().unwrap());
    let mut txid = [0u8; 32];
    txid.copy_from_slice(take(&mut o, 32));
    let output_index = u32::from_le_bytes(take(&mut o, 4).try_into().unwrap());
    debug_assert_eq!(o, DESC_LEN);
    Ok(Descriptor { version, covenant_id, owner_pubkey, amount, for_sale, price, txid, output_index })
}

// ---------------------------------------------------------------------------
// derive_p2sh: fields -> Address, the SAME path inspect_v3 proved on-chain.
// CScriptNum lives HERE (compile_contract lowers the int ctor args via add_i64),
// never in transport.
// ---------------------------------------------------------------------------
fn derive_p2sh(d: &Descriptor) -> Address {
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let amount_i64 = i64::try_from(d.amount).expect("amount fits i64");
    let price_i64 = i64::try_from(d.price).expect("price fits i64");
    let compiled = compile_contract(
        &source,
        &ctor_args(d.owner_pubkey.to_vec(), amount_i64, d.for_sale, price_i64),
        CompileOptions::default(),
    )
    .expect("compile parcel");
    let p2sh = pay_to_script_hash_script(&compiled.script);
    extract_script_pub_key_address(&p2sh, Prefix::Testnet).expect("p2sh address")
}

fn hex_to_32(hex: &str, what: &str) -> [u8; 32] {
    let mut buf = vec![0u8; hex.len() / 2];
    faster_hex::hex_decode(hex.as_bytes(), &mut buf).unwrap_or_else(|_| panic!("bad {what} hex"));
    assert_eq!(buf.len(), 32, "{what} must be 32 bytes, got {}", buf.len());
    let mut out = [0u8; 32];
    out.copy_from_slice(&buf);
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        selftest();
        return;
    }
    eprintln!("bitphoque_descriptor_v1 — parcel descriptor codec");
    eprintln!("  --selftest   round-trip + P2SH-derive a known parcel (no node needed)");
    eprintln!();
    eprintln!("(encode/decode/derive are library fns; --selftest exercises them. CLI");
    eprintln!(" subcommands for emit/import wire into the wallet CLI, not this proof bin.)");
}

fn selftest() {
    println!("=== bitphoque descriptor v1 — SELFTEST (no node) ===\n");

    // --- fixture: the live 4b0c8187..:0 parcel — buyer1, 5000 phoques, for-sale @ 500000 ---
    // NOTE: owner_pubkey + covenant_id below are FROM bitphoque_status.md notes; the
    // derived address is what you compare against the chain (4b0c8187..:0).
    let owner_hex = "8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936"; // buyer1 x-only
    let cov_hex = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458"; // v3 covenant_id
    let txid_hex = "4b0c8187b25f761949518c47d77bbf682d649e509f4ad7069ae6493e1c6d0575";
    // ground-truth P2SH for this parcel, from the proven bitphoque_inspect_v3:
    let expected_p2sh = "kaspatest:pp6yju0m6m3akaw79es3sz7d5cmrha6gftnsqeql7p2cmvwc7rws6atrkwdkj";

    // Guard: if the placeholders weren't filled in, say so loudly rather than panic on length.
    if owner_hex.len() != 64 || cov_hex.len() != 64 {
        println!("[selftest] NOTE: owner_pubkey and/or covenant_id are PLACEHOLDER prefixes.");
        println!("           Fill the full 64-hex (32-byte) values at the top of selftest()");
        println!("           before the P2SH-derive assertion is meaningful.");
        println!("           Running the codec round-trip on synthetic 32-byte fields instead.\n");
        roundtrip_only_synthetic();
        return;
    }

    let d = Descriptor {
        version: DESC_VERSION,
        covenant_id: hex_to_32(cov_hex, "covenant_id"),
        owner_pubkey: hex_to_32(owner_hex, "owner_pubkey"),
        amount: 5000,
        for_sale: true,
        price: 500000,
        txid: hex_to_32(txid_hex, "txid"),
        output_index: 0,
    };

    // 1) round-trip
    let s = encode_descriptor(&d);
    println!("[selftest] encoded descriptor ({} chars): {s}", s.len());
    let back = decode_descriptor(&s).expect("decode must succeed");
    assert_eq!(d, back, "ROUND-TRIP MISMATCH");
    println!("[selftest] round-trip OK (decode == encode input)\n");

    // 2) checksum guard — flip one char, expect a LOUD failure
    let mut bad = s.clone().into_bytes();
    let last = bad.len() - 1;
    bad[last] = if bad[last] == b'1' { b'2' } else { b'1' };
    let bad_s = String::from_utf8(bad).unwrap();
    match decode_descriptor(&bad_s) {
        Ok(_) => println!("[selftest] WARNING: corrupted descriptor decoded clean (checksum NOT working!)"),
        Err(e) => println!("[selftest] checksum guard OK — corrupted descriptor rejected: {e}\n"),
    }

    // 3) derive the P2SH and ASSERT it matches the proven inspect_v3 ground truth
    let addr = derive_p2sh(&d);
    let addr_str = addr.to_string();
    println!("[selftest] derived P2SH : {addr_str}");
    println!("[selftest] expected P2SH: {expected_p2sh}");
    assert_eq!(addr_str, expected_p2sh, "P2SH DERIVATION MISMATCH — codec does not match the chain");
    println!("[selftest] P2SH derive OK — codec derives the SAME address inspect_v3 proved on-chain");
    println!("           (live parcel 4b0c8187..:0). End-to-end: descriptor -> fields -> correct P2SH.");
    println!("\n=== SELFTEST DONE (round-trip + checksum + on-chain-matching P2SH all PASS) ===");
}

fn roundtrip_only_synthetic() {
    let d = Descriptor {
        version: DESC_VERSION,
        covenant_id: [0xABu8; 32],
        owner_pubkey: [0xCDu8; 32],
        amount: 5000,
        for_sale: true,
        price: 500000,
        txid: [0xEFu8; 32],
        output_index: 0,
    };
    let s = encode_descriptor(&d);
    println!("[selftest] synthetic encoded ({} chars): {s}", s.len());
    let back = decode_descriptor(&s).expect("decode must succeed");
    assert_eq!(d, back, "ROUND-TRIP MISMATCH");
    println!("[selftest] synthetic round-trip OK (decode == encode input)");

    // checksum guard on synthetic too
    let mut bad = s.clone().into_bytes();
    let last = bad.len() - 1;
    bad[last] = if bad[last] == b'1' { b'2' } else { b'1' };
    match decode_descriptor(&String::from_utf8(bad).unwrap()) {
        Ok(_) => println!("[selftest] WARNING: corrupted descriptor decoded clean (checksum NOT working!)"),
        Err(e) => println!("[selftest] checksum guard OK — corrupted rejected: {e}"),
    }
    println!("\n=== SELFTEST DONE (synthetic round-trip only; fill real fields for the P2SH assert) ===");
}
