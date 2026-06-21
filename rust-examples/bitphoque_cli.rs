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
use kaspa_txscript::{pay_to_script_hash_script, extract_script_pub_key_address, script_builder::ScriptBuilder, EngineCtx, EngineFlags, TxScriptEngine};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::Keypair;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CovenantDeclCallOptions};

const NODE: &str = "192.168.0.206:16210";
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

#[tokio::main]
async fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let go = raw.iter().any(|a| a == "--go");
    let pos: Vec<&String> = raw.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.is_empty() {
        eprintln!("usage: bitphoque <import|balance|list|split|partial-list> ...");
        eprintln!("  import       <owner_pk_hex> <amount> <forSale 0|1> <price>");
        eprintln!("  balance      <owner_pk_hex>");
        eprintln!("  list         <owner_keyfile> <amount> <price> [--go]");
        eprintln!("  split        <owner_keyfile> <in_amount> <out0_amount> [--go]");
        eprintln!("  partial-list <owner_keyfile> <in_amount> <list_amount> <price> [--go]");
        std::process::exit(2);
    }
    let cmd = pos[0].as_str();
    let rest = &pos[1..];
    match cmd {
        "import"       => cmd_import(rest).await,
        "balance"      => cmd_balance(rest).await,
        "list"         => cmd_list(rest, go).await,
        "split"        => cmd_split(rest, go).await,
        "partial-list" => cmd_partial_list(rest, go).await,
        other => { eprintln!("unknown subcommand: {other}"); std::process::exit(2); }
    }
}
