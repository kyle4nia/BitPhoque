// BitPhoque — CLI (thin slice: import / balance / list / partial-list). TN10 practice.
//
// First collapse of the proven harnesses into one party-agnostic binary, built on the Option-C
// wallet model: the wallet TRACKS ITS OWN PARCELS FORWARD in a local wallet.json instead of trying
// to scan the DAG. Each parcel record carries exactly the ctor args that determine its P2SH
// (owner, amount, forSale, price, caps) plus its on-chain outpoint + value, so the wallet can always
// recompute the P2SH and confirm the parcel live via get_utxos_by_addresses — it never has to GUESS
// an amount (the indexing problem the harnesses dodged and a real wallet otherwise hits).
//
// wallet.json: plain JSON at C:\bitphoque\wallet.json (gitignored, local state — NOT source).
//   NEVER contains private keys. owner_pk is public; signing always reads a *.key file by PATH.
//   Safe to have on screen while streaming.
//
// SUBCOMMANDS (this slice):
//   import  <owner_pk_hex> <amount> <forSale 0|1> <price>
//       Confirm a parcel on-chain (inspect query) and record it in wallet.json. The Option-C
//       bootstrap + rescan-by-hint recovery path: you tell it what you know you own, it verifies
//       and records. (Needed because parcels created by the old harnesses predate the wallet.)
//   balance <owner_pk_hex>
//       Read wallet.json, recompute each owned parcel's P2SH, confirm live against the node,
//       print holdings (total + per-parcel, for-sale flagged). Pure read. Prunes spent records.
//   list <owner_keyfile> <amount> <price> [--go]
//       List a WHOLE parcel for sale. 1-in / 1-out, owner-signed. The degenerate case of
//       partial-list (no change output, no value split): the entire balance flips forSale=true
//       at a price. On ACCEPTED: marks the input parcel spent + records the one new for-sale parcel.
//   partial-list <owner_keyfile> <in_amount> <list_amount> <price> [--go]
//       Lift of the proven bitphoque_partial_list_v3 harness. Lists list_amount @ price (out0,
//       for-sale) + keeps (in_amount - list_amount) (out1, change), same owner, one owner-signed tx.
//       On ACCEPTED: marks the input parcel spent + records BOTH new parcels in wallet.json.
//
// Same safety spine as every harness: dry-run by default, --go to broadcast, LOCAL VM VERIFY gates
// --go. Single input, leader ABI. TN10 PRACTICE ONLY.

use std::collections::BTreeMap;
use std::fs;

use kaspa_addresses::{Prefix, Version, Address};
use kaspa_consensus_core::{ mass::{ComputeBudget, ScriptUnits},
    constants::TX_VERSION_TOCCATA,
    hashing::sighash::{calc_schnorr_signature_hash, SigHashReusedValuesUnsync},
    hashing::sighash_type::SIG_HASH_ALL,
    subnets::SUBNETWORK_ID_NATIVE,
    tx::{
        CovenantBinding, MutableTransaction, PopulatedTransaction, Transaction, TransactionInput, VerifiableTransaction,
        TransactionOutpoint, TransactionOutput, TxInputMass, UtxoEntry,
    },
};
use kaspa_grpc_client::GrpcClient;
use kaspa_hashes::Hash;
use kaspa_notify::subscription::context::SubscriptionContext;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_txscript::caches::Cache;
use kaspa_txscript::covenants::CovenantsContext;
use kaspa_txscript::{pay_to_script_hash_script, pay_to_address_script, extract_script_pub_key_address, script_builder::ScriptBuilder, EngineCtx, EngineFlags, TxScriptEngine};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::Keypair;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CovenantDeclCallOptions};

const NODE: &str = "192.168.0.104:16210";  // TestnetNode gRPC, all-ifaces (--rpclisten=0.0.0.0:16210). 16211 is P2P; 17210 is wRPC.
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";
const WALLET_FILE: &str = r"C:\bitphoque\wallet.json";

const FEE: u64 = 700_000;
const MAX_COV_INS: i64 = 3;
const MAX_COV_OUTS: i64 = 3;
const COVENANT_SCRIPT_UNITS: u64 = 240_000;
const MIN_OUT_VALUE: u64 = 1_000_000;
const INDEX_LAG_RETRIES: u32 = 8;       // address-index lags tx-index by a few seconds
const INDEX_LAG_SLEEP_MS: u64 = 1_500;

// ----------------------------------------------------------------------------------------------
// wallet.json — minimal hand-rolled JSON (no serde dep needed; records are flat and few).
// A parcel record is exactly the P2SH-determining ctor args + on-chain locator + value + status.
// ----------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Parcel {
    outpoint_txid: String,   // hex txid
    outpoint_index: u32,
    owner_pk: String,        // 32-byte x-only hex (PUBLIC — never a private key)
    amount: i64,
    for_sale: bool,
    price: i64,
    value: u64,              // TKAS sompi in the UTXO
    max_ins: i64,
    max_outs: i64,
    status: String,          // "live" | "spent"
}

impl Parcel {
    // Stable key so re-import / re-record updates rather than duplicates.
    fn key(&self) -> String { format!("{}:{}", self.outpoint_txid, self.outpoint_index) }

    fn to_json(&self) -> String {
        format!(
            "{{\"outpoint_txid\":\"{}\",\"outpoint_index\":{},\"owner_pk\":\"{}\",\"amount\":{},\"for_sale\":{},\"price\":{},\"value\":{},\"max_ins\":{},\"max_outs\":{},\"status\":\"{}\"}}",
            self.outpoint_txid, self.outpoint_index, self.owner_pk, self.amount,
            self.for_sale, self.price, self.value, self.max_ins, self.max_outs, self.status
        )
    }
}

// ----------------------------------------------------------------------------------------------
// PARCEL DESCRIPTOR CODEC — copied VERBATIM from the proven bitphoque_descriptor_v1.rs (committed
// dedc62d, --selftest PASSED). The wire FORMAT is FINALIZED (Option B, 2026-06-23): a flat
// fixed-offset 118-byte payload, base58check-wrapped. Frozen format => safe to duplicate (it will
// not drift). encode/decode are the transport half; P2SH derivation reuses the CLI's existing
// confirm_parcel machinery (ctor_args -> compile_contract -> pay_to_script_hash_script), so the
// descriptor's covenant_id is checked but the address is recomputed locally, never trusted from wire.
//
// Layout: version(1) | covenant_id(32) | owner_pubkey x-only(32) | amount u64-LE(8)
//         | forSale(1) | price u64-LE(8) | txid(32) | output_index u32-LE(4) = 118 B
// ----------------------------------------------------------------------------------------------

const DESC_VERSION: u8 = 1;
const DESC_LEN: usize = 118;

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

fn encode_payload(d: &Descriptor) -> Vec<u8> {
    let mut out = Vec::with_capacity(DESC_LEN);
    out.push(d.version);
    out.extend_from_slice(&d.covenant_id);
    out.extend_from_slice(&d.owner_pubkey);
    out.extend_from_slice(&d.amount.to_le_bytes());
    out.push(if d.for_sale { 1 } else { 0 });
    out.extend_from_slice(&d.price.to_le_bytes());
    out.extend_from_slice(&d.txid);
    out.extend_from_slice(&d.output_index.to_le_bytes());
    debug_assert_eq!(out.len(), DESC_LEN, "payload must be exactly {DESC_LEN} bytes");
    out
}

#[allow(dead_code)]
fn encode_descriptor(d: &Descriptor) -> String {
    bs58::encode(encode_payload(d)).with_check().into_string()
}

fn decode_descriptor(s: &str) -> Result<Descriptor, String> {
    let payload = bs58::decode(s.trim())
        .with_check(None)
        .into_vec()
        .map_err(|e| format!("base58check decode failed (bad checksum or alphabet): {e}"))?;
    if payload.len() != DESC_LEN {
        return Err(format!("descriptor payload is {} bytes, expected {DESC_LEN}", payload.len()));
    }
    let mut o = 0usize;
    let take = |o: &mut usize, n: usize| -> &[u8] { let s = &payload[*o..*o + n]; *o += n; s };
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

// Tiny tolerant parser for the flat objects we write (NOT a general JSON parser — only reads our own
// output). Splits the top-level array on "},{" and pulls fields by name.
fn load_wallet() -> BTreeMap<String, Parcel> {
    let mut map = BTreeMap::new();
    let raw = match fs::read_to_string(WALLET_FILE) {
        Ok(s) => s,
        Err(_) => return map, // no wallet yet -> empty
    };
    let trimmed = raw.trim();
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() { return map; }
    // Normalize the record separator: save_wallet joins with "},\n {" for readability, but a record
    // boundary is any "}" + whitespace + "{". Collapse all such boundaries to a bare "},{" so the
    // split below matches regardless of formatting (and survives a human hand-editing the file).
    let mut normalized = String::with_capacity(inner.len());
    let bytes = inner.as_bytes();
    let mut k = 0usize;
    while k < bytes.len() {
        if bytes[k] == b'}' {
            // look ahead past the separator (comma + any whitespace) for a '{'
            let mut j = k + 1;
            while j < bytes.len() && (bytes[j] == b',' || (bytes[j] as char).is_whitespace()) { j += 1; }
            if j < bytes.len() && bytes[j] == b'{' {
                normalized.push_str("},{");
                k = j + 1;
                continue;
            }
        }
        normalized.push(bytes[k] as char);
        k += 1;
    }
    for chunk in normalized.split("},{") {
        let obj = chunk.trim().trim_start_matches('{').trim_end_matches('}');
        let get_str = |field: &str| -> String {
            let pat = format!("\"{}\":\"", field);
            if let Some(start) = obj.find(&pat) {
                let rest = &obj[start + pat.len()..];
                if let Some(end) = rest.find('"') { return rest[..end].to_string(); }
            }
            String::new()
        };
        let get_raw = |field: &str| -> String {
            let pat = format!("\"{}\":", field);
            if let Some(start) = obj.find(&pat) {
                let rest = &obj[start + pat.len()..];
                let end = rest.find(|c| c == ',' || c == '}').unwrap_or(rest.len());
                return rest[..end].trim().trim_matches('"').to_string();
            }
            String::new()
        };
        let p = Parcel {
            outpoint_txid: get_str("outpoint_txid"),
            outpoint_index: get_raw("outpoint_index").parse().unwrap_or(0),
            owner_pk: get_str("owner_pk"),
            amount: get_raw("amount").parse().unwrap_or(0),
            for_sale: get_raw("for_sale") == "true",
            price: get_raw("price").parse().unwrap_or(0),
            value: get_raw("value").parse().unwrap_or(0),
            max_ins: get_raw("max_ins").parse().unwrap_or(MAX_COV_INS),
            max_outs: get_raw("max_outs").parse().unwrap_or(MAX_COV_OUTS),
            status: get_str("status"),
        };
        if !p.outpoint_txid.is_empty() { map.insert(p.key(), p); }
    }
    map
}

fn save_wallet(map: &BTreeMap<String, Parcel>) {
    let body: Vec<String> = map.values().map(|p| p.to_json()).collect();
    let json = format!("[{}]", body.join(",\n "));
    fs::write(WALLET_FILE, json).expect("write wallet.json");
}

// ----------------------------------------------------------------------------------------------
// Shared spine (lifted verbatim from the proven harnesses — copied, not redesigned).
// ----------------------------------------------------------------------------------------------

fn ctor_args(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner), Expr::int(amount), Expr::byte(0), Expr::bool(false),
        Expr::bool(for_sale), Expr::int(price), Expr::int(MAX_COV_INS), Expr::int(MAX_COV_OUTS),
    ]
}

fn new_state(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Expr<'static> {
    struct_object(vec![
        ("ownerIdentifier", Expr::bytes(owner)), ("identifierType", Expr::byte(0)),
        ("amount", Expr::int(amount)), ("isMinter", Expr::bool(false)),
        ("forSale", Expr::bool(for_sale)), ("price", Expr::int(price)),
    ])
}

fn load_keypair(path: &str) -> Keypair {
    let hex = fs::read_to_string(path).expect("read key file");
    let mut sk = [0u8; 32];
    faster_hex::hex_decode(hex.trim().as_bytes(), &mut sk).expect("bad hex key");
    Keypair::from_seckey_slice(secp256k1::SECP256K1, &sk).expect("bad key")
}

fn decode_pubkey(hex: &str) -> Vec<u8> {
    let mut pk = [0u8; 32];
    faster_hex::hex_decode(hex.trim().as_bytes(), &mut pk).expect("bad pubkey hex (want 32-byte x-only)");
    pk.to_vec()
}

fn covenant_id() -> Hash {
    let mut b = [0u8; 32];
    faster_hex::hex_decode(COVENANT_ID_HEX.as_bytes(), &mut b).expect("bad cov id hex");
    Hash::from_bytes(b)
}

fn local_vm_verify(tx: &Transaction, entries: Vec<UtxoEntry>, input_idx: usize) -> Result<(), TxScriptError> {
    let reused_values = SigHashReusedValuesUnsync::new();
    let sig_cache = Cache::new(10_000);
    let input = tx.inputs[input_idx].clone();
    let populated = PopulatedTransaction::new(tx, entries);
    let cov_ctx = CovenantsContext::from_tx(&populated).map_err(TxScriptError::from)?;
    let utxo = populated.utxo(input_idx).expect("selected input utxo");
    let mut vm = TxScriptEngine::from_transaction_input(
        &populated, &input, input_idx, utxo,
        EngineCtx::new(&sig_cache).with_reused(&reused_values).with_covenants_ctx(&cov_ctx),
        EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() },
    );
    vm.execute()
}

// Compile a parcel's P2SH from ctor args and query the node for live UTXOs at it, retrying through
// the address-index lag. Returns (outpoint, value) of the first matching UTXO if found.
async fn confirm_parcel(rpc: &GrpcClient, owner_pk: &[u8], amount: i64, for_sale: bool, price: i64)
    -> Option<(TransactionOutpoint, u64)> {
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let compiled = compile_contract(&source, &ctor_args(owner_pk.to_vec(), amount, for_sale, price), CompileOptions::default()).expect("compile parcel");
    let p2sh = pay_to_script_hash_script(&compiled.script);
    let addr = extract_script_pub_key_address(&p2sh, Prefix::Testnet).expect("p2sh address");
    for attempt in 1..=INDEX_LAG_RETRIES {
        let resp = rpc.get_utxos_by_addresses(vec![addr.clone()]).await.expect("get_utxos failed");
        if let Some(entry) = resp.first() {
            let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
            return Some((outpoint, entry.utxo_entry.amount));
        }
        if attempt < INDEX_LAG_RETRIES {
            tokio::time::sleep(std::time::Duration::from_millis(INDEX_LAG_SLEEP_MS)).await;
        }
    }
    None
}

async fn connect() -> GrpcClient {
    let subscription_context = SubscriptionContext::new();
    println!("[cli] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[cli] connected.");
    rpc
}

// ----------------------------------------------------------------------------------------------
// Subcommands
// ----------------------------------------------------------------------------------------------

async fn cmd_import(args: &[&String]) {
    if args.len() < 4 {
        eprintln!("usage: bitphoque import <owner_pk_hex> <amount> <forSale 0|1> <price>");
        std::process::exit(2);
    }
    let owner_pk = decode_pubkey(args[0]);
    let amount: i64 = args[1].parse().expect("amount must be an integer");
    let for_sale = match args[2].as_str() { "1" => true, "0" => false, _ => { eprintln!("forSale must be 0 or 1"); std::process::exit(2); } };
    let price: i64 = args[3].parse().expect("price must be an integer");

    println!("[import] parcel: owner {} amount {amount} forSale {for_sale} price {price} caps {MAX_COV_INS}/{MAX_COV_OUTS}", args[0]);
    let rpc = connect().await;
    match confirm_parcel(&rpc, &owner_pk, amount, for_sale, price).await {
        Some((outpoint, value)) => {
            let p = Parcel {
                outpoint_txid: outpoint.transaction_id.to_string(),
                outpoint_index: outpoint.index,
                owner_pk: args[0].trim().to_string(),
                amount, for_sale, price, value,
                max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS,
                status: "live".to_string(),
            };
            println!("[import] CONFIRMED on-chain: {}:{}  value {} sompi", p.outpoint_txid, p.outpoint_index, p.value);
            let mut wallet = load_wallet();
            wallet.insert(p.key(), p);
            save_wallet(&wallet);
            println!("[import] recorded in wallet.json.");
        }
        None => println!("[import] NOT FOUND on-chain after retries (owner/amount/forSale/price/caps must match a live parcel). nothing recorded."),
    }
}

// receive <descriptor>
//   Decode a base58check parcel descriptor handed over by a sender, VERIFY it (checksum via decode,
//   version, covenant_id against our known token), CONFIRM the parcel is live on-chain, and record it
//   in wallet.json. This is the descriptor-driven sibling of `import`: same end-state (a confirmed
//   parcel in the wallet) but the four parcel fields arrive in one paste-safe string instead of being
//   hand-typed, and the descriptor additionally carries covenant_id/txid/index for integrity.
//   Chain stays the authority: we recompute the P2SH locally (confirm_parcel) and never trust the
//   descriptor's claimed outpoint blindly — a descriptor for a spent/non-existent parcel records nothing.
async fn cmd_receive(args: &[&String]) {
    if args.is_empty() {
        eprintln!("usage: bitphoque receive <descriptor>");
        std::process::exit(2);
    }
    let d = match decode_descriptor(args[0]) {
        Ok(d) => d,
        Err(e) => { eprintln!("[receive] BAD descriptor — {e}"); std::process::exit(2); }
    };

    // version gate
    if d.version != DESC_VERSION {
        eprintln!("[receive] descriptor version {} not supported (this build expects v{DESC_VERSION}).", d.version);
        std::process::exit(2);
    }

    // covenant_id integrity tie: refuse a descriptor for a different token rather than derive a wrong P2SH.
    let mut our_cov = [0u8; 32];
    faster_hex::hex_decode(COVENANT_ID_HEX.as_bytes(), &mut our_cov).expect("bad cov id hex const");
    if d.covenant_id != our_cov {
        let mut got = vec![0u8; 64];
        faster_hex::hex_encode(&d.covenant_id, &mut got).ok();
        eprintln!("[receive] REFUSED — descriptor covenant_id does not match this token.");
        eprintln!("          descriptor: {}", String::from_utf8_lossy(&got));
        eprintln!("          expected  : {COVENANT_ID_HEX}");
        std::process::exit(2);
    }

    // amount/price are non-negative on the wire (u64); confirm_parcel + ctor want i64.
    let amount = i64::try_from(d.amount).expect("amount fits i64");
    let price = i64::try_from(d.price).expect("price fits i64");
    let owner_pk = d.owner_pubkey.to_vec();
    let mut owner_hex = vec![0u8; 64];
    faster_hex::hex_encode(&d.owner_pubkey, &mut owner_hex).ok();
    let owner_hex = String::from_utf8(owner_hex).unwrap();

    println!("[receive] descriptor OK: owner {owner_hex} amount {amount} forSale {} price {price} caps {MAX_COV_INS}/{MAX_COV_OUTS}", d.for_sale);
    println!("[receive] confirming parcel live on-chain (chain is the authority; descriptor is only a hint)...");
    let rpc = connect().await;
    match confirm_parcel(&rpc, &owner_pk, amount, d.for_sale, price).await {
        Some((outpoint, value)) => {
            let p = Parcel {
                outpoint_txid: outpoint.transaction_id.to_string(),
                outpoint_index: outpoint.index,
                owner_pk: owner_hex,
                amount, for_sale: d.for_sale, price, value,
                max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS,
                status: "live".to_string(),
            };
            println!("[receive] CONFIRMED on-chain: {}:{}  value {} sompi", p.outpoint_txid, p.outpoint_index, p.value);
            let mut wallet = load_wallet();
            wallet.insert(p.key(), p);
            save_wallet(&wallet);
            println!("[receive] recorded in wallet.json.");
        }
        None => println!("[receive] NOT FOUND on-chain after retries — the descriptor may name a spent or non-existent parcel, or the node is still syncing. nothing recorded."),
    }
}

async fn cmd_balance(args: &[&String]) {
    if args.is_empty() {
        eprintln!("usage: bitphoque balance <owner_pk_hex>");
        std::process::exit(2);
    }
    let want_owner = args[0].trim().to_string();
    let owner_pk = decode_pubkey(&want_owner);
    let owner_addr = Address::new(Prefix::Testnet, Version::PubKey, &owner_pk);
    println!("[balance] owner {want_owner}");
    println!("[balance] P2PK addr: {owner_addr}");

    let wallet = load_wallet();
    let mine: Vec<Parcel> = wallet.values().filter(|p| p.owner_pk == want_owner && p.status == "live").cloned().collect();
    if mine.is_empty() {
        println!("[balance] no live parcels for this owner in wallet.json. (use `import` to seed parcels created outside the wallet.)");
        return;
    }

    let rpc = connect().await;
    let mut total: i64 = 0;
    let mut for_sale_total: i64 = 0;
    let mut unconfirmed = 0;
    println!("[balance] confirming {} recorded parcel(s) against the node...", mine.len());
    for p in &mine {
        // A recorded-live parcel always counts toward the balance. A lookup MISS is AMBIGUOUS
        // (address-index lag under load vs. a genuine spend), so balance NEVER writes status on a
        // miss — it only flags "not confirmed right now". Pruning genuinely-spent records is an
        // explicit action, not a side effect of a flaky query. (This is the fix for the bug where a
        // lagging index made balance wrongly mark fresh parcels spent.)
        let tag = if p.for_sale { format!("FOR SALE @ {}", p.price) } else { "not-for-sale".to_string() };
        match confirm_parcel(&rpc, &owner_pk, p.amount, p.for_sale, p.price).await {
            Some((outpoint, value)) => {
                println!("  {:>8} phoques  [{}]  {}:{}  ({} sompi)  confirmed", p.amount, tag, outpoint.transaction_id, outpoint.index, value);
            }
            None => {
                println!("  {:>8} phoques  [{}]  {}:{}  NOT confirmed right now (index lag or spent — left unchanged)", p.amount, tag, p.outpoint_txid, p.outpoint_index);
                unconfirmed += 1;
            }
        }
        total += p.amount;
        if p.for_sale { for_sale_total += p.amount; }
    }
    println!("[balance] TOTAL: {total} phoques ({for_sale_total} listed for sale, {} held)", total - for_sale_total);
    if unconfirmed > 0 {
        println!("[balance] note: {unconfirmed} parcel(s) not confirmed on this pass (node mempool can lag the index under load). re-run to recheck; nothing was changed.");
    }
}

// List a WHOLE parcel for sale. 1-in / 1-out, owner-signed. The degenerate case of partial-list:
// the entire balance flips to forSale=true at a price — no change output, no proportional value
// split. On-chain proven shape (v3 listings cebf75b5 / 55542a99; v2 cad1220f). Minimal diff off
// cmd_partial_list: three args, one output, value = in_value - FEE, one-element new_states.
async fn cmd_list(args: &[&String], go: bool) {
    if args.len() < 3 {
        eprintln!("usage: bitphoque list <owner_keyfile> <amount> <price> [--go]");
        std::process::exit(2);
    }
    let owner_key_file = args[0].as_str();
    let amount: i64 = args[1].parse().expect("amount must be an integer");
    let price: i64 = args[2].parse().expect("price must be an integer");

    assert!(amount > 0, "amount must be > 0");
    assert!(price > 0, "price must be > 0");
    println!("[list] listing WHOLE parcel of {amount} phoques @ {price} (out0, FOR SALE)");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);

    let signer = load_keypair(owner_key_file);
    let owner_pk = signer.x_only_public_key().0.serialize().to_vec();
    let owner_hex = faster_hex::hex_string(&owner_pk);
    println!("[list] owner (signs, owns the output): {owner_hex}");

    // Input parcel: this amount, NOT for sale (price 0). Output: same amount, FOR SALE @ price.
    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);
    let out0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount, true, price), CompileOptions::default()).expect("compile out0");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);

    let rpc = connect().await;
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[list] looking for not-for-sale cov UTXO ({amount} phoques) at {token_addr}");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() { println!("[list] no covenant UTXO at this P2SH. aborting."); return; }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[list] spending cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);
    if in_value <= FEE { println!("[list] value {in_value} <= fee {FEE}. aborting."); return; }

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));

    // Whole spendable balance carries into the single for-sale parcel (no split).
    let val0: u64 = in_value - FEE;
    println!("[list] output value: {val0} sompi (fee {FEE})");
    if val0 < MIN_OUT_VALUE {
        println!("[list] output below the {MIN_OUT_VALUE} sompi floor (out0 {val0}). aborting."); return;
    }

    let output0 = TransactionOutput { value: val0, script_public_key: out0_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    let outputs = vec![output0];

    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = signer.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    let new_states: Expr = vec![
        new_state(owner_pk.clone(), amount, true, price),
    ].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();
    let txid = tx.id().to_string();
    println!("[list] list tx id: {txid}");

    println!("[list] local VM verify...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[list] LOCAL VM VERIFY: PASS"),
        Err(e) => { println!("[list] LOCAL VM VERIFY: FAIL — {e:?}. aborting."); return; }
    }

    if !go {
        println!("\n[list] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[list] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => {
            println!("[list] ACCEPTED. node response: {resp:?}");
            // Record forward: input parcel spent, one new for-sale parcel live (out0 idx 0).
            let mut wallet = load_wallet();
            let in_key = format!("{}:{}", tx.inputs[0].previous_outpoint.transaction_id, tx.inputs[0].previous_outpoint.index);
            if let Some(rec) = wallet.get_mut(&in_key) { rec.status = "spent".to_string(); }
            let p0 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 0, owner_pk: owner_hex.clone(), amount, for_sale: true, price, value: val0, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            wallet.insert(p0.key(), p0);
            save_wallet(&wallet);
            println!("[list] wallet.json updated: input marked spent; out0 (for-sale {amount} @ {price}) recorded.");
        }
        Err(e) => println!("[list] REJECTED: {e}"),
    }
}

async fn cmd_partial_list(args: &[&String], go: bool) {
    if args.len() < 4 {
        eprintln!("usage: bitphoque partial-list <owner_keyfile> <in_amount> <list_amount> <price> [--go]");
        std::process::exit(2);
    }
    let owner_key_file = args[0].as_str();
    let in_amount: i64 = args[1].parse().expect("in_amount must be an integer");
    let list_amount: i64 = args[2].parse().expect("list_amount must be an integer");
    let price: i64 = args[3].parse().expect("price must be an integer");

    assert!(list_amount > 0, "list_amount must be > 0");
    assert!(list_amount < in_amount, "list_amount must be < in_amount (lists PART of the balance; use `list` for the whole parcel)");
    assert!(price > 0, "price must be > 0");
    let keep_amount = in_amount - list_amount;
    println!("[plist] in {in_amount} -> list {list_amount} @ {price} (out0, FOR SALE) + keep {keep_amount} (out1, not-for-sale)");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);

    let signer = load_keypair(owner_key_file);
    let owner_pk = signer.x_only_public_key().0.serialize().to_vec();
    let owner_hex = faster_hex::hex_string(&owner_pk);
    println!("[plist] owner (signs, owns both outputs): {owner_hex}");

    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), in_amount, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);
    let out0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), list_amount, true, price), CompileOptions::default()).expect("compile out0");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);
    let out1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), keep_amount, false, 0), CompileOptions::default()).expect("compile out1");
    let out1_p2sh = pay_to_script_hash_script(&out1_compiled.script);

    let rpc = connect().await;
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[plist] looking for not-for-sale cov UTXO ({in_amount} phoques) at {token_addr}");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() { println!("[plist] no covenant UTXO at this P2SH. aborting."); return; }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[plist] spending cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);
    if in_value <= FEE { println!("[plist] value {in_value} <= fee {FEE}. aborting."); return; }

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));

    let spendable = in_value - FEE;
    let val0: u64 = ((spendable as u128 * list_amount as u128) / in_amount as u128) as u64;
    let val1: u64 = spendable - val0;
    println!("[plist] value split (proportional): out0 {val0} sompi, out1 {val1} sompi (fee {FEE})");
    if val0 < MIN_OUT_VALUE || val1 < MIN_OUT_VALUE {
        println!("[plist] an output is below the {MIN_OUT_VALUE} sompi floor (out0 {val0}, out1 {val1}). aborting."); return;
    }

    let output0 = TransactionOutput { value: val0, script_public_key: out0_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    let output1 = TransactionOutput { value: val1, script_public_key: out1_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    let outputs = vec![output0, output1];

    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = signer.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    let new_states: Expr = vec![
        new_state(owner_pk.clone(), list_amount, true, price),
        new_state(owner_pk.clone(), keep_amount, false, 0),
    ].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();
    let txid = tx.id().to_string();
    println!("[plist] partial-list tx id: {txid}");

    println!("[plist] local VM verify...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[plist] LOCAL VM VERIFY: PASS"),
        Err(e) => { println!("[plist] LOCAL VM VERIFY: FAIL — {e:?}. aborting."); return; }
    }

    if !go {
        println!("\n[plist] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[plist] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => {
            println!("[plist] ACCEPTED. node response: {resp:?}");
            // Record forward: input parcel spent, two new parcels live (out0 idx 0, out1 idx 1).
            let mut wallet = load_wallet();
            let in_key = format!("{}:{}", tx.inputs[0].previous_outpoint.transaction_id, tx.inputs[0].previous_outpoint.index);
            if let Some(rec) = wallet.get_mut(&in_key) { rec.status = "spent".to_string(); }
            let p0 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 0, owner_pk: owner_hex.clone(), amount: list_amount, for_sale: true,  price, value: val0, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            let p1 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 1, owner_pk: owner_hex.clone(), amount: keep_amount, for_sale: false, price: 0, value: val1, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            wallet.insert(p0.key(), p0);
            wallet.insert(p1.key(), p1);
            save_wallet(&wallet);
            println!("[plist] wallet.json updated: input marked spent; out0 (for-sale {list_amount}) + out1 (keep {keep_amount}) recorded.");
        }
        Err(e) => println!("[plist] REJECTED: {e}"),
    }
}

// Split ONE parcel into TWO, both NOT-for-sale, same owner. 1-in / 2-out, owner-signed. The
// not-for-sale sibling of partial-list: instead of "list part, keep rest", it's "carve one holding
// into two holdings you still own" (making change, or seeding multiple parcels from one — the on-
// chain 8a–8d splits). Minimal diff off cmd_partial_list: drop the price, both outputs forSale=false.
// out1_amount is derived (in_amount - out0_amount) so conservation is automatic.
async fn cmd_split(args: &[&String], go: bool) {
    if args.len() < 3 {
        eprintln!("usage: bitphoque split <owner_keyfile> <in_amount> <out0_amount> [--go]");
        std::process::exit(2);
    }
    let owner_key_file = args[0].as_str();
    let in_amount: i64 = args[1].parse().expect("in_amount must be an integer");
    let out0_amount: i64 = args[2].parse().expect("out0_amount must be an integer");

    assert!(out0_amount > 0, "out0_amount must be > 0");
    assert!(out0_amount < in_amount, "out0_amount must be < in_amount (this is a SPLIT into two non-empty parcels)");
    let out1_amount = in_amount - out0_amount;
    println!("[split] in {in_amount} -> out0 {out0_amount} + out1 {out1_amount} (both NOT-for-sale, same owner)");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);

    let signer = load_keypair(owner_key_file);
    let owner_pk = signer.x_only_public_key().0.serialize().to_vec();
    let owner_hex = faster_hex::hex_string(&owner_pk);
    println!("[split] owner (signs, owns both outputs): {owner_hex}");

    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), in_amount, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);
    let out0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), out0_amount, false, 0), CompileOptions::default()).expect("compile out0");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);
    let out1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), out1_amount, false, 0), CompileOptions::default()).expect("compile out1");
    let out1_p2sh = pay_to_script_hash_script(&out1_compiled.script);

    let rpc = connect().await;
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[split] looking for not-for-sale cov UTXO ({in_amount} phoques) at {token_addr}");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() { println!("[split] no covenant UTXO at this P2SH. aborting."); return; }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[split] spending cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);
    if in_value <= FEE { println!("[split] value {in_value} <= fee {FEE}. aborting."); return; }

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));

    let spendable = in_value - FEE;
    let val0: u64 = ((spendable as u128 * out0_amount as u128) / in_amount as u128) as u64;
    let val1: u64 = spendable - val0;
    println!("[split] value split (proportional): out0 {val0} sompi, out1 {val1} sompi (fee {FEE})");
    if val0 < MIN_OUT_VALUE || val1 < MIN_OUT_VALUE {
        println!("[split] an output is below the {MIN_OUT_VALUE} sompi floor (out0 {val0}, out1 {val1}). aborting."); return;
    }

    let output0 = TransactionOutput { value: val0, script_public_key: out0_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    let output1 = TransactionOutput { value: val1, script_public_key: out1_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    let outputs = vec![output0, output1];

    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = signer.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    let new_states: Expr = vec![
        new_state(owner_pk.clone(), out0_amount, false, 0),
        new_state(owner_pk.clone(), out1_amount, false, 0),
    ].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();
    let txid = tx.id().to_string();
    println!("[split] split tx id: {txid}");

    println!("[split] local VM verify...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[split] LOCAL VM VERIFY: PASS"),
        Err(e) => { println!("[split] LOCAL VM VERIFY: FAIL — {e:?}. aborting."); return; }
    }

    if !go {
        println!("\n[split] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[split] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => {
            println!("[split] ACCEPTED. node response: {resp:?}");
            // Record forward: input parcel spent, two new not-for-sale parcels live (out0 idx 0, out1 idx 1).
            let mut wallet = load_wallet();
            let in_key = format!("{}:{}", tx.inputs[0].previous_outpoint.transaction_id, tx.inputs[0].previous_outpoint.index);
            if let Some(rec) = wallet.get_mut(&in_key) { rec.status = "spent".to_string(); }
            let p0 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 0, owner_pk: owner_hex.clone(), amount: out0_amount, for_sale: false, price: 0, value: val0, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            let p1 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 1, owner_pk: owner_hex.clone(), amount: out1_amount, for_sale: false, price: 0, value: val1, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            wallet.insert(p0.key(), p0);
            wallet.insert(p1.key(), p1);
            save_wallet(&wallet);
            println!("[split] wallet.json updated: input marked spent; out0 ({out0_amount}) + out1 ({out1_amount}) recorded.");
        }
        Err(e) => println!("[split] REJECTED: {e}"),
    }
}

// Merge TWO parcels into ONE, both NOT-for-sale, same owner. 2-in / 1-out, owner-signed on BOTH
// inputs. The inverse of split. This is the ONLY subcommand with multiple covenant inputs, so it
// carries the leader/delegate ABI and the two-sigs-against-index-0 detail — ported verbatim from
// the proven bitphoque_sale_merge_v3 harness (on-chain 519063be...). DO NOT reason these from
// scratch; they are match-proven:
//   - input 0 = LEADER (full arrays: new_states=[merged], sigs=[sig0,sig1], witnesses=[0,1]).
//   - input 1 = DELEGATE (empty args, is_leader:false), reveals its own redeem script.
//   - BOTH sigs computed against input INDEX 0 (leader position) — signing sig1 against input 1
//     yields a different sighash and the covenant rejects.
//   - witnesses=[0,1] is DORMANT FILLER (no for-sale input -> checkAuth never indexes it); matched
//     to the proven kcc20 pattern rather than reasoned about.
// Merge needs HEAVIER fee + script units than the single-input ops (two full 2793B script reveals),
// so it uses LOCAL constants here — bumping the shared FEE/COVENANT_SCRIPT_UNITS would break
// list/split/partial-list (calibrated to 700k / 240k).
async fn cmd_merge(args: &[&String], go: bool) {
    const MERGE_FEE: u64 = 1_300_000;            // node-confirmed floor ~1,244,500; 1.3M for margin.
    const MERGE_SCRIPT_UNITS: u64 = 300_000;     // two covenant inputs, each a full script reveal.

    if args.len() < 3 {
        eprintln!("usage: bitphoque merge <owner_keyfile> <amount0> <amount1> [--go]");
        std::process::exit(2);
    }
    let owner_key_file = args[0].as_str();
    let amount0: i64 = args[1].parse().expect("amount0 must be an integer");
    let amount1: i64 = args[2].parse().expect("amount1 must be an integer");

    assert!(amount0 > 0 && amount1 > 0, "both amounts must be > 0");
    let out_amount = amount0 + amount1;
    println!("[merge] merging {amount0} + {amount1} = {out_amount} phoques (both not-for-sale, same owner)");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let cov_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(MERGE_SCRIPT_UNITS)).expect("budget fits");
    let cov_mass = TxInputMass::ComputeBudget(cov_budget);

    let signer = load_keypair(owner_key_file);
    let owner_pk = signer.x_only_public_key().0.serialize().to_vec();
    let owner_hex = faster_hex::hex_string(&owner_pk);
    println!("[merge] owner (signs BOTH inputs): {owner_hex}");

    // Two input contracts (same owner, not-for-sale, each amount -> different P2SH) + one output.
    let in0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount0, false, 0), CompileOptions::default()).expect("compile in0");
    let in0_p2sh = pay_to_script_hash_script(&in0_compiled.script);
    let in1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount1, false, 0), CompileOptions::default()).expect("compile in1");
    let in1_p2sh = pay_to_script_hash_script(&in1_compiled.script);
    let out_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), out_amount, false, 0), CompileOptions::default()).expect("compile out");
    let out_p2sh = pay_to_script_hash_script(&out_compiled.script);

    let rpc = connect().await;

    let in0_addr = extract_script_pub_key_address(&in0_p2sh, Prefix::Testnet).expect("in0 addr");
    println!("[merge] looking for parcel 0 ({amount0} phoques, not-for-sale) at {in0_addr}");
    let in0_resp = rpc.get_utxos_by_addresses(vec![in0_addr]).await.expect("get_utxos in0");
    if in0_resp.is_empty() { println!("[merge] no not-for-sale UTXO for parcel 0 ({amount0}). aborting."); return; }
    let in0_entry = &in0_resp[0];
    let in0_outpoint = TransactionOutpoint::from(in0_entry.outpoint.clone());
    let in0_value = in0_entry.utxo_entry.amount;
    println!("[merge] parcel 0 utxo {}:{}  value {}", in0_outpoint.transaction_id, in0_outpoint.index, in0_value);

    let in1_addr = extract_script_pub_key_address(&in1_p2sh, Prefix::Testnet).expect("in1 addr");
    println!("[merge] looking for parcel 1 ({amount1} phoques, not-for-sale) at {in1_addr}");
    let in1_resp = rpc.get_utxos_by_addresses(vec![in1_addr]).await.expect("get_utxos in1");
    if in1_resp.is_empty() { println!("[merge] no not-for-sale UTXO for parcel 1 ({amount1}). aborting."); return; }
    let in1_entry = &in1_resp[0];
    let in1_outpoint = TransactionOutpoint::from(in1_entry.outpoint.clone());
    let in1_value = in1_entry.utxo_entry.amount;
    println!("[merge] parcel 1 utxo {}:{}  value {}", in1_outpoint.transaction_id, in1_outpoint.index, in1_value);

    let cov_id = covenant_id();
    let in0_utxo = UtxoEntry::new(in0_value, in0_p2sh.clone(), 0, false, Some(cov_id));
    let in1_utxo = UtxoEntry::new(in1_value, in1_p2sh.clone(), 0, false, Some(cov_id));

    if in0_value + in1_value <= MERGE_FEE { println!("[merge] combined value <= fee {MERGE_FEE}. aborting."); return; }
    let out_value = in0_value + in1_value - MERGE_FEE;
    if out_value < MIN_OUT_VALUE { println!("[merge] output below the {MIN_OUT_VALUE} sompi floor ({out_value}). aborting."); return; }
    let output = TransactionOutput { value: out_value, script_public_key: out_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    println!("[merge] output: {out_amount} phoques, value {out_value} (= {in0_value} + {in1_value} - fee {MERGE_FEE})");

    // Unsigned skeleton (both inputs empty) for the covenant sighash. BOTH sigs sign INDEX 0.
    let in0u = TransactionInput { previous_outpoint: in0_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let in1u = TransactionInput { previous_outpoint: in1_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0u, in1u], vec![output.clone()], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let entries = vec![in0_utxo.clone(), in1_utxo.clone()];
    let mtx = MutableTransaction::with_entries(unsigned, entries.clone());
    let reused = SigHashReusedValuesUnsync::new();

    // CRITICAL (proven): both signatures are computed against the LEADER input (index 0).
    let sh0 = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg0 = secp256k1::Message::from_digest_slice(sh0.as_bytes().as_slice()).expect("sighash0");
    let mut sig0 = signer.sign_schnorr(msg0).as_ref().to_vec();
    sig0.push(SIG_HASH_ALL.to_u8());
    let mut sig1 = signer.sign_schnorr(msg0).as_ref().to_vec();   // ALSO signs index 0
    sig1.push(SIG_HASH_ALL.to_u8());

    // Leader (input 0) carries the complete arrays; delegate (input 1) is empty.
    let new_states: Expr = vec![new_state(owner_pk.clone(), out_amount, false, 0)].into();
    let sigs: Expr = vec![Expr::bytes(sig0), Expr::bytes(sig1)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8, 1u8]);   // dormant filler, match-proven
    let mut in0_ss = in0_compiled.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("build leader sigscript");
    let redeem0 = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in0_compiled.script).expect("push redeem0").drain();
    in0_ss.extend_from_slice(&redeem0);

    let mut in1_ss = in1_compiled.build_sig_script_for_covenant_decl("transfer", vec![], CovenantDeclCallOptions { is_leader: false }).expect("build delegate sigscript");
    let redeem1 = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in1_compiled.script).expect("push redeem1").drain();
    in1_ss.extend_from_slice(&redeem1);

    let in0 = TransactionInput { previous_outpoint: in0_outpoint, signature_script: in0_ss, sequence: 0, mass: cov_mass };
    let in1 = TransactionInput { previous_outpoint: in1_outpoint, signature_script: in1_ss, sequence: 0, mass: cov_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0, in1], vec![output], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();
    let txid = tx.id().to_string();
    println!("[merge] merge tx id: {txid}");

    // VM-verify BOTH covenant inputs.
    println!("[merge] local VM verify (input 0)...");
    match local_vm_verify(&tx, entries.clone(), 0) {
        Ok(()) => println!("[merge] input 0: PASS"),
        Err(e) => { println!("[merge] input 0: FAIL — {e:?}. aborting."); return; }
    }
    println!("[merge] local VM verify (input 1)...");
    match local_vm_verify(&tx, entries.clone(), 1) {
        Ok(()) => println!("[merge] input 1: PASS — both signed inputs accepted, 2->1 conserved."),
        Err(e) => { println!("[merge] input 1: FAIL — {e:?}. aborting."); return; }
    }

    if !go {
        println!("\n[merge] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[merge] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => {
            println!("[merge] ACCEPTED. node response: {resp:?}");
            // Record forward: BOTH input parcels spent, one merged not-for-sale parcel live (idx 0).
            let mut wallet = load_wallet();
            let in0_key = format!("{}:{}", tx.inputs[0].previous_outpoint.transaction_id, tx.inputs[0].previous_outpoint.index);
            let in1_key = format!("{}:{}", tx.inputs[1].previous_outpoint.transaction_id, tx.inputs[1].previous_outpoint.index);
            if let Some(rec) = wallet.get_mut(&in0_key) { rec.status = "spent".to_string(); }
            if let Some(rec) = wallet.get_mut(&in1_key) { rec.status = "spent".to_string(); }
            let p0 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 0, owner_pk: owner_hex.clone(), amount: out_amount, for_sale: false, price: 0, value: out_value, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            wallet.insert(p0.key(), p0);
            save_wallet(&wallet);
            println!("[merge] wallet.json updated: both inputs marked spent; merged out0 ({out_amount}) recorded.");
        }
        Err(e) => println!("[merge] REJECTED: {e}"),
    }
}
async fn cmd_claim(args: &[&String], go: bool) {
    // CLAIM a for-sale listing: pay the seller >= price, receive the phoques. NO seller key needed
    // (payment-authorized) — that is the whole point of the sale design. The covenant input is
    // UNSIGNED; a second plain input (the buyer's funds) pays the seller. Ported verbatim from the
    // proven bitphoque_sale_claim_v3 harness (on-chain 3265e063 / 9ffe84b8). Single covenant input,
    // so NO leader/delegate complexity and NO leader-index-0 sig quirk: the lone covenant input is
    // leader, and the buyer's PLAIN input signs against ITS OWN index (1), the normal sighash.
    //   input 0: for-sale covenant UTXO. UNSIGNED. dummy 65B sig. witness -> payment output index 1.
    //   input 1: buyer plain funding. buyer-SIGNED over input index 1.
    //   output 0: token -> buyer (covenant, forSale reset to false, same amount).
    //   output 1: payment -> seller P2PK (PAYMENT >= price; PAYMENT dodges KIP-9 dust, status doc #6).
    //   output 2: change -> buyer (plain).
    const CLAIM_FEE: u64 = 900_000;          // v3 2-input claim: node floor 857k (mass 8570) -> 900k margin.
    const CLAIM_SCRIPT_UNITS: u64 = 300_000; // one covenant script reveal + one plain input.
    const PAYMENT: u64 = 100_000_000;        // 1 TKAS, >> typical price AND above the dust threshold.

    if args.len() < 4 {
        eprintln!("usage: bitphoque claim <buyer_keyfile> <seller_pubkey_hex> <amount> <price> [--go]");
        std::process::exit(2);
    }
    let buyer_key_file = args[0].as_str();
    let seller_pk = decode_pubkey(args[1]);
    let amount: i64 = args[2].parse().expect("amount must be an integer (e.g. 21034)");
    let price: i64 = args[3].parse().expect("price must be an integer (e.g. 500000)");
    if PAYMENT < price as u64 {
        eprintln!("[claim] PAYMENT {PAYMENT} < price {price}; contract requires payment >= price. aborting.");
        std::process::exit(2);
    }

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let cov_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(CLAIM_SCRIPT_UNITS)).expect("budget fits");
    let cov_mass = TxInputMass::ComputeBudget(cov_budget);
    let plain_mass = TxInputMass::ComputeBudget(11.into());

    let buyer = load_keypair(buyer_key_file);
    let buyer_pk = buyer.x_only_public_key().0.serialize().to_vec();
    let buyer_hex = faster_hex::hex_string(&buyer_pk);
    let seller_addr = Address::new(Prefix::Testnet, Version::PubKey, &seller_pk);
    let buyer_addr = Address::new(Prefix::Testnet, Version::PubKey, &buyer_pk);
    println!("[claim] seller (payout) pubkey: {}", faster_hex::hex_string(&seller_pk));
    println!("[claim] buyer: {buyer_hex}");
    println!("[claim] amount {amount}  price {price}  PAYMENT {PAYMENT}  fee {CLAIM_FEE}");
    println!("[claim] seller payout addr: {seller_addr}");
    println!("[claim] buyer addr: {buyer_addr}");

    // For-sale INPUT contract = seller-owned, forSale=true, price=price, at amount.
    let forsale_compiled = compile_contract(&source, &ctor_args(seller_pk.clone(), amount, true, price), CompileOptions::default()).expect("compile for-sale");
    let forsale_p2sh = pay_to_script_hash_script(&forsale_compiled.script);
    // OUTPUT token contract = BUYER-owned, forSale=false (claim resets the flag), same amount.
    let buyer_token_compiled = compile_contract(&source, &ctor_args(buyer_pk.clone(), amount, false, 0), CompileOptions::default()).expect("compile buyer token");
    let buyer_token_p2sh = pay_to_script_hash_script(&buyer_token_compiled.script);

    let rpc = connect().await;

    // Find the for-sale covenant UTXO (by-address lookup; the only available UTXO query on this client).
    let forsale_addr = extract_script_pub_key_address(&forsale_p2sh, Prefix::Testnet).expect("fs p2sh address");
    println!("[claim] looking for for-sale cov UTXO at {forsale_addr}");
    let fs_resp = rpc.get_utxos_by_addresses(vec![forsale_addr]).await.expect("get_utxos fs");
    if fs_resp.is_empty() {
        println!("[claim] no for-sale covenant UTXO found (seller/amount/price/caps must match the listing). aborting.");
        return;
    }
    let fs_entry = &fs_resp[0];
    let fs_outpoint = TransactionOutpoint::from(fs_entry.outpoint.clone());
    let fs_value = fs_entry.utxo_entry.amount;
    println!("[claim] for-sale cov utxo {}:{}  value {}", fs_outpoint.transaction_id, fs_outpoint.index, fs_value);

    // Find a buyer funding UTXO (plain P2PK).
    let buyer_spk = pay_to_address_script(&buyer_addr);
    let b_resp = rpc.get_utxos_by_addresses(vec![buyer_addr.clone()]).await.expect("get_utxos buyer");
    if b_resp.is_empty() { println!("[claim] no buyer funding UTXO found. fund {buyer_addr} first. aborting."); return; }
    let b_entry = &b_resp[0];
    let b_outpoint = TransactionOutpoint::from(b_entry.outpoint.clone());
    let b_value = b_entry.utxo_entry.amount;
    println!("[claim] buyer funding utxo {}:{}  value {}", b_outpoint.transaction_id, b_outpoint.index, b_value);
    if b_value <= PAYMENT + CLAIM_FEE { println!("[claim] buyer funds {b_value} <= payment+fee {}. aborting.", PAYMENT + CLAIM_FEE); return; }

    let cov_id = covenant_id();
    let fs_utxo = UtxoEntry::new(fs_value, forsale_p2sh.clone(), 0, false, Some(cov_id));
    let b_utxo = UtxoEntry::new(b_value, buyer_spk.clone(), 0, false, None);

    // Outputs: token->buyer (covenant carries full value), payment->seller (plain), change->buyer (plain).
    let token_out = TransactionOutput {
        value: fs_value,
        script_public_key: buyer_token_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };
    let payment_out = TransactionOutput { value: PAYMENT, script_public_key: pay_to_address_script(&seller_addr), covenant: None };
    let change_value = b_value - PAYMENT - CLAIM_FEE;
    if change_value < MIN_OUT_VALUE { println!("[claim] change below the {MIN_OUT_VALUE} sompi floor ({change_value}). aborting."); return; }
    let change_out = TransactionOutput { value: change_value, script_public_key: buyer_spk.clone(), covenant: None };
    let outputs = vec![token_out, payment_out, change_out];

    // Unsigned skeleton (both inputs empty) to compute the buyer's sighash on input 1.
    let in0_unsigned = TransactionInput { previous_outpoint: fs_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let in1_unsigned = TransactionInput { previous_outpoint: b_outpoint, signature_script: vec![], sequence: 0, mass: plain_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0_unsigned, in1_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);

    // Sign INPUT 1 (buyer's plain funding) over its OWN index (1) — single covenant input, so no leader quirk.
    let mtx = MutableTransaction::with_entries(unsigned, vec![fs_utxo.clone(), b_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let b_sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 1, SIG_HASH_ALL, &reused);
    let b_msg = secp256k1::Message::from_digest_slice(b_sig_hash.as_bytes().as_slice()).expect("buyer sighash");
    let b_sig = buyer.sign_schnorr(b_msg);
    let mut b_signature = b_sig.as_ref().to_vec();
    b_signature.push(SIG_HASH_ALL.to_u8());
    let in1_sigscript = ScriptBuilder::new().add_data(&b_signature).expect("push buyer sig").drain();

    // INPUT 0 covenant leader sigscript: new-state (buyer-owned, forSale=false), dummy sig, witness->output 1.
    let new_states: Expr = vec![new_state(buyer_pk.clone(), amount, false, 0)].into();
    let sigs: Expr = vec![Expr::bytes(vec![0u8; 65])].into(); // dummy; for-sale branch is payment-authorized
    let witnesses: Expr = Expr::bytes(vec![1u8]);             // payment output index (output 1)
    let mut in0_sigscript = forsale_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&forsale_compiled.script).expect("push redeem").drain();
    in0_sigscript.extend_from_slice(&redeem_push);

    let in0 = TransactionInput { previous_outpoint: fs_outpoint, signature_script: in0_sigscript, sequence: 0, mass: cov_mass };
    let in1 = TransactionInput { previous_outpoint: b_outpoint, signature_script: in1_sigscript, sequence: 0, mass: plain_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0, in1], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();
    let txid = tx.id().to_string();
    println!("[claim] claim tx id: {txid}");
    println!("[claim] buyer pays {PAYMENT} to seller, receives {amount} phoques; change {change_value}; fee {CLAIM_FEE}");

    // LOCAL VM VERIFY the COVENANT input (input 0) — the payment-authorized spend.
    println!("[claim] local VM verify (covenant input 0)...");
    match local_vm_verify(&tx, vec![fs_utxo.clone(), b_utxo.clone()], 0) {
        Ok(()) => println!("[claim] LOCAL VM VERIFY: PASS — the covenant accepts this claim."),
        Err(e) => { println!("[claim] LOCAL VM VERIFY: FAIL — {e:?}. aborting."); return; }
    }

    if !go {
        println!("\n[claim] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[claim] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => {
            println!("[claim] ACCEPTED. node response: {resp:?}");
            // Record forward: for-sale input spent; new buyer-owned not-for-sale parcel live (idx 0).
            let mut wallet = load_wallet();
            let in_key = format!("{}:{}", tx.inputs[0].previous_outpoint.transaction_id, tx.inputs[0].previous_outpoint.index);
            if let Some(rec) = wallet.get_mut(&in_key) { rec.status = "spent".to_string(); }
            let p0 = Parcel { outpoint_txid: txid.clone(), outpoint_index: 0, owner_pk: buyer_hex.clone(), amount, for_sale: false, price: 0, value: fs_value, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
            wallet.insert(p0.key(), p0);
            save_wallet(&wallet);
            println!("[claim] wallet.json updated: for-sale input spent; claimed out0 ({amount}, buyer-owned) recorded.");
        }
        Err(e) => println!("[claim] REJECTED: {e}"),
    }
}

// send <signer_keyfile> <in_amount> <send_amount> <recipient_pubkey_hex> [--go]
//   Transfer phoques to a DIFFERENT owner. The transfer half is a minimal diff off cmd_split — same
//   single-input, owner-signed, leader-ABI path — with ONE change: out0's owner is the RECIPIENT's
//   pubkey, not the signer's. Two cases:
//     PARTIAL (send_amount < in_amount): 2-out. out0 -> recipient (send_amount), out1 -> signer change
//        (in_amount - send_amount). Both not-for-sale. Proven shape (== bitphoque_split_transfer_v3).
//     WHOLE   (send_amount == in_amount): 1-out. out0 -> recipient (whole parcel). No change output.
//        The 1-out covenant shape is proven (original bitphoque_transfer is 1-in/1-out); the v3
//        leader-ABI single-element new_states is gated by the LOCAL VM VERIFY below before any --go.
//   On --go ACCEPTED: the signer's CHANGE parcel (if any) is recorded in wallet.json; the recipient's
//   out0 is NOT recorded (not ours) — instead a parcel DESCRIPTOR for out0 is emitted to hand over.
//   The recipient runs `receive <descriptor>` to confirm + record it. send produces what receive consumes.
async fn cmd_send(args: &[&String], go: bool) {
    if args.len() < 4 {
        eprintln!("usage: bitphoque send <signer_keyfile> <in_amount> <send_amount> <recipient_pubkey_hex> [--go]");
        std::process::exit(2);
    }
    let signer_key_file = args[0].as_str();
    let in_amount: i64 = args[1].parse().expect("in_amount must be an integer");
    let send_amount: i64 = args[2].parse().expect("send_amount must be an integer");
    let recipient_pk = decode_pubkey(args[3]);
    let recipient_hex = faster_hex::hex_string(&recipient_pk);

    assert!(send_amount > 0, "send_amount must be > 0");
    assert!(send_amount <= in_amount, "send_amount must be <= in_amount");
    let whole = send_amount == in_amount;
    let change_amount = in_amount - send_amount; // 0 in the whole-parcel case
    if whole {
        println!("[send] WHOLE parcel: in {in_amount} -> recipient {send_amount} (1-out, no change)");
    } else {
        println!("[send] PARTIAL: in {in_amount} -> recipient {send_amount} (out0) + change {change_amount} (out1)");
    }

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);

    let signer = load_keypair(signer_key_file);
    let owner_pk = signer.x_only_public_key().0.serialize().to_vec();
    let owner_hex = faster_hex::hex_string(&owner_pk);
    println!("[send] signer (input owner): {owner_hex}");
    println!("[send] recipient (out0 owner): {recipient_hex}");

    // Input contract: signer-owned, not-for-sale, at in_amount.
    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), in_amount, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);
    // out0 = recipient-owned, not-for-sale, send_amount.
    let out0_compiled = compile_contract(&source, &ctor_args(recipient_pk.clone(), send_amount, false, 0), CompileOptions::default()).expect("compile out0");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);

    let rpc = connect().await;
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[send] looking for not-for-sale cov UTXO ({in_amount} phoques) at {token_addr}");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() { println!("[send] no covenant UTXO at this P2SH (signer/in_amount/caps must match). aborting."); return; }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[send] spending cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);
    if in_value <= FEE { println!("[send] value {in_value} <= fee {FEE}. aborting."); return; }

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));
    let spendable = in_value - FEE;

    // Build outputs + parallel new_states. WHOLE -> 1 output; PARTIAL -> 2 outputs (out0 recipient, out1 change).
    let (outputs, new_states_vec, val0, val1): (Vec<TransactionOutput>, Vec<Expr>, u64, u64) = if whole {
        if spendable < MIN_OUT_VALUE { println!("[send] sole output {spendable} below {MIN_OUT_VALUE} floor. aborting."); return; }
        let o0 = TransactionOutput { value: spendable, script_public_key: out0_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
        (vec![o0], vec![new_state(recipient_pk.clone(), send_amount, false, 0)], spendable, 0)
    } else {
        // proportional TKAS split, same as cmd_split; out0 = recipient share.
        let v0: u64 = ((spendable as u128 * send_amount as u128) / in_amount as u128) as u64;
        let v1: u64 = spendable - v0;
        if v0 < MIN_OUT_VALUE || v1 < MIN_OUT_VALUE { println!("[send] an output below {MIN_OUT_VALUE} floor (out0 {v0}, out1 {v1}). aborting."); return; }
        let out1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), change_amount, false, 0), CompileOptions::default()).expect("compile out1");
        let out1_p2sh = pay_to_script_hash_script(&out1_compiled.script);
        let o0 = TransactionOutput { value: v0, script_public_key: out0_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
        let o1 = TransactionOutput { value: v1, script_public_key: out1_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
        (vec![o0, o1],
         vec![new_state(recipient_pk.clone(), send_amount, false, 0), new_state(owner_pk.clone(), change_amount, false, 0)],
         v0, v1)
    };
    if whole { println!("[send] value: out0 {val0} sompi (fee {FEE})"); }
    else     { println!("[send] value split (proportional): out0 {val0} sompi, out1 {val1} sompi (fee {FEE})"); }

    // Sign input 0 (not-for-sale -> signature path), leader ABI — identical to cmd_split.
    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = signer.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    let new_states: Expr = new_states_vec.into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]); // dormant on signed branch; ABI expects one entry per input
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();
    let txid = tx.id().to_string();
    println!("[send] tx id: {txid}");

    println!("[send] local VM verify...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[send] LOCAL VM VERIFY: PASS"),
        Err(e) => { println!("[send] LOCAL VM VERIFY: FAIL — {e:?}. aborting."); return; }
    }

    if !go {
        println!("\n[send] DRY RUN — not submitted. Re-run with --go to broadcast (the recipient descriptor prints only after broadcast).");
        return;
    }

    println!("[send] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => {
            println!("[send] ACCEPTED. node response: {resp:?}");
            let mut wallet = load_wallet();
            // input parcel spent
            let in_key = format!("{}:{}", tx.inputs[0].previous_outpoint.transaction_id, tx.inputs[0].previous_outpoint.index);
            if let Some(rec) = wallet.get_mut(&in_key) { rec.status = "spent".to_string(); }
            // record OUR change parcel (partial only); out0 belongs to the recipient -> NOT recorded.
            if !whole {
                let pc = Parcel { outpoint_txid: txid.clone(), outpoint_index: 1, owner_pk: owner_hex.clone(), amount: change_amount, for_sale: false, price: 0, value: val1, max_ins: MAX_COV_INS, max_outs: MAX_COV_OUTS, status: "live".to_string() };
                wallet.insert(pc.key(), pc);
                println!("[send] wallet.json updated: input spent; change parcel ({change_amount}) recorded (out0 is the recipient's, not recorded).");
            } else {
                println!("[send] wallet.json updated: input spent (whole parcel sent; no change).");
            }
            save_wallet(&wallet);

            // Emit the descriptor for out0 (the recipient's parcel) — what they feed to `receive`.
            let mut cov_arr = [0u8; 32];
            faster_hex::hex_decode(COVENANT_ID_HEX.as_bytes(), &mut cov_arr).expect("cov id");
            let mut owner_arr = [0u8; 32];
            owner_arr.copy_from_slice(&recipient_pk);
            let mut txid_arr = [0u8; 32];
            txid_arr.copy_from_slice(tx.id().as_bytes().as_slice());
            let desc = Descriptor {
                version: DESC_VERSION,
                covenant_id: cov_arr,
                owner_pubkey: owner_arr,
                amount: send_amount as u64,
                for_sale: false,
                price: 0,
                txid: txid_arr,
                output_index: 0,
            };
            let desc_str = encode_descriptor(&desc);
            println!("\n[send] === RECIPIENT DESCRIPTOR (hand this to {recipient_hex}) ===");
            println!("{desc_str}");
            println!("[send] === they run: bitphoque receive <descriptor> ===");
        }
        Err(e) => println!("[send] REJECTED: {e}"),
    }
}

#[tokio::main]
async fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let go = raw.iter().any(|a| a == "--go");
    let pos: Vec<&String> = raw.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.is_empty() {
        eprintln!("usage: bitphoque <import|receive|balance|list|split|merge|partial-list|claim> ...");
        eprintln!("  import       <owner_pk_hex> <amount> <forSale 0|1> <price>");
        eprintln!("  receive      <descriptor>          (decode + verify + confirm + record a handed-over parcel)");
        eprintln!("  balance      <owner_pk_hex>");
        eprintln!("  list         <owner_keyfile> <amount> <price> [--go]");
        eprintln!("  split        <owner_keyfile> <in_amount> <out0_amount> [--go]");
        eprintln!("  merge        <owner_keyfile> <amount0> <amount1> [--go]");
        eprintln!("  partial-list <owner_keyfile> <in_amount> <list_amount> <price> [--go]");
        eprintln!("  claim        <buyer_keyfile> <seller_pubkey_hex> <amount> <price> [--go]");
        eprintln!("  send         <signer_keyfile> <in_amount> <send_amount> <recipient_pubkey_hex> [--go]");
        std::process::exit(2);
    }
    let cmd = pos[0].as_str();
    let rest = &pos[1..];
    match cmd {
        "import"       => cmd_import(rest).await,
        "receive"      => cmd_receive(rest).await,
        "balance"      => cmd_balance(rest).await,
        "list"         => cmd_list(rest, go).await,
        "split"        => cmd_split(rest, go).await,
        "merge"        => cmd_merge(rest, go).await,
        "partial-list" => cmd_partial_list(rest, go).await,
        "claim"        => cmd_claim(rest, go).await,
        "send"         => cmd_send(rest, go).await,
        other => { eprintln!("unknown subcommand: {other}"); std::process::exit(2); }
    }
}
