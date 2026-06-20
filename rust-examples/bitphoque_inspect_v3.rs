// BitPhoque — INSPECT v3 (generalized, read-only). Confirm an arbitrary v3 parcel on-chain.
//
// Unlike the original bitphoque_inspect.rs (which had ONE token P2SH baked in), this tool
// COMPILES the v3 P2SH in-process from CLI params — owner pubkey + amount + forSale + price —
// exactly the way bitphoque_sale_list_v3.rs computes the P2SH it must find. Because every
// (owner, amount, forSale, price, caps) tuple is a DIFFERENT P2SH, you tell it which parcel
// you want to confirm and it derives that parcel's address, then queries the node.
//
// READ-ONLY: no keys, no signing, no transactions. It takes the owner PUBKEY (hex), never a
// keyfile — confirming a UTXO's existence needs only the public identity. Stream-safe.
//
// INDEXING NOTE (verified against rusty-kaspa RpcApi source): the node exposes NO by-outpoint
// UTXO query — get_utxos_by_addresses is the only UTXO lookup. The address index lags the
// tx-index by a few seconds after a confirmation, so this tool RETRIES the by-address lookup
// (--retries N). An optional --outpoint filters the returned set to one exact txid:index
// CLIENT-SIDE (the outpoint is a filter on results, not a query to the node).
//
// Because it links the silverscript compiler API (compile_contract) to build the P2SH, this is
// a silverscript-clone bin (like list/claim), NOT a rothschild bin like the old inspect.
//
// Build:  cargo build -p silverscript-lang --bin bitphoque_inspect_v3
// CLI:    <owner_pubkey_hex> <amount> <forsale:0|1> <price> [--outpoint <txid:index>] [--retries N]
//   e.g.  ... -- 8c02c20a...4936 20033 0 0
//         ... -- 8c02c20a...4936 20033 0 0 --outpoint 4f084e26...:1 --retries 5

use std::fs;

use kaspa_addresses::{Prefix, Version, Address};
use kaspa_grpc_client::GrpcClient;
use kaspa_notify::subscription::context::SubscriptionContext;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_txscript::{pay_to_script_hash_script, extract_script_pub_key_address};
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, CompileOptions};

const NODE: &str = "192.168.0.206:16210";
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";

const MAX_COV_INS: i64 = 3;     // MUST match genesis + split (baked into P2SH)
const MAX_COV_OUTS: i64 = 3;

// 8-arg ctor: owner, amount, idType=0, isMinter=false, forSale, price, maxIns, maxOuts.
// Identical shape to the list harness so the SAME P2SH is computed.
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

fn parse_outpoint(s: &str) -> (String, u32) {
    let (tx, idx) = s.rsplit_once(':').expect("outpoint must be txid:index");
    (tx.to_string(), idx.parse().expect("outpoint index must be an integer"))
}

#[tokio::main]
async fn main() {
    // CLI: <owner_pubkey_hex> <amount> <forsale:0|1> <price> [--outpoint txid:index] [--retries N]
    let raw: Vec<String> = std::env::args().collect();
    let pos: Vec<&String> = raw.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.len() < 4 {
        eprintln!("usage: bitphoque_inspect_v3 <owner_pubkey_hex> <amount> <forsale:0|1> <price> [--outpoint txid:index] [--retries N]");
        eprintln!("  e.g. 8c02c20a..4936 20033 0 0");
        std::process::exit(2);
    }
    let owner_hex = pos[0].as_str();
    let amount: i64 = pos[1].parse().expect("amount must be an integer (e.g. 20033)");
    let for_sale: bool = match pos[2].as_str() { "0" => false, "1" => true, _ => panic!("forsale must be 0 or 1") };
    let price: i64 = pos[3].parse().expect("price must be an integer (0 if not-for-sale)");

    // optional flags
    let mut want_outpoint: Option<(String, u32)> = None;
    let mut retries: u32 = 3;
    let mut it = raw.iter().skip(1).peekable();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--outpoint" => want_outpoint = Some(parse_outpoint(it.next().expect("--outpoint needs a value"))),
            "--retries"  => retries = it.next().expect("--retries needs a value").parse().expect("retries must be an integer"),
            _ => {}
        }
    }

    // owner pubkey (x-only, hex) -> bytes
    let mut owner_pk = vec![0u8; owner_hex.len() / 2];
    faster_hex::hex_decode(owner_hex.as_bytes(), &mut owner_pk).expect("bad owner pubkey hex");
    if owner_pk.len() != 32 { panic!("owner pubkey must be 32 bytes (x-only), got {}", owner_pk.len()); }

    println!("[inspect-v3] parcel: owner {owner_hex}");
    println!("[inspect-v3]         amount {amount}  forSale {for_sale}  price {price}  caps {MAX_COV_INS}/{MAX_COV_OUTS}");
    let owner_addr = Address::new(Prefix::Testnet, Version::PubKey, &owner_pk);
    println!("[inspect-v3]         owner P2PK addr: {owner_addr}");

    // Compile THIS parcel's P2SH in-process (same ctor shape as the list harness).
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount, for_sale, price), CompileOptions::default()).expect("compile parcel");
    let p2sh = pay_to_script_hash_script(&compiled.script);
    let token_addr = extract_script_pub_key_address(&p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[inspect-v3] computed parcel P2SH: {token_addr}");
    println!("[inspect-v3] expected covenant_id : {COVENANT_ID_HEX}");

    let subscription_context = SubscriptionContext::new();
    println!("[inspect-v3] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[inspect-v3] connected.");

    // by-address lookup WITH RETRY (the address index lags the tx-index a few seconds).
    let mut resp = Vec::new();
    for attempt in 1..=retries.max(1) {
        resp = rpc.get_utxos_by_addresses(vec![token_addr.clone()]).await.expect("get_utxos failed");
        if !resp.is_empty() { break; }
        if attempt < retries {
            println!("[inspect-v3] attempt {attempt}/{retries}: none yet, waiting 3s for the address index to catch up...");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    if resp.is_empty() {
        println!();
        println!("[inspect-v3] NO UTXOs at this parcel P2SH after {retries} attempt(s).");
        println!("             (check: amount correct? forSale/price correct? caps 3/3? already spent?)");
        return;
    }

    // optional client-side outpoint filter
    let entries: Vec<_> = match &want_outpoint {
        Some((tx, idx)) => resp.iter().filter(|e| e.outpoint.transaction_id.to_string() == *tx && e.outpoint.index == *idx).cloned().collect(),
        None => resp.clone(),
    };

    if let Some((tx, idx)) = &want_outpoint {
        if entries.is_empty() {
            println!();
            println!("[inspect-v3] parcel P2SH has {} UTXO(s) but NONE match outpoint {tx}:{idx}.", resp.len());
            println!("             (that exact UTXO is spent or never existed at this P2SH.)");
            return;
        }
    }

    println!();
    println!("[inspect-v3] FOUND {} matching UTXO(s) at the parcel P2SH:", entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let op = &entry.outpoint;
        let amt = entry.utxo_entry.amount;
        println!("  [{i}] outpoint : {}:{}", op.transaction_id, op.index);
        println!("      value    : {amt} sompi  ({amount} phoques in this parcel)");
    }
    println!();
    println!("[inspect-v3] confirmed on-chain.");
}
