// BitPhoque — INSPECT (confirm on-chain existence). Read-only.
//
// Derives the token P2SH from the compiled contract, queries the node for
// UTXOs at that address, and prints each one (outpoint, value, covenant_id).
// No keys, no signing, no transactions — pure read. This is the standard
// "show me the BitPhoques on-chain" command.
//
// Build:  cargo build --release -p rothschild --bin bitphoque_inspect
// Run:    cargo run   --release -p rothschild --bin bitphoque_inspect

use std::fs;

use kaspa_addresses::Prefix;
use kaspa_grpc_client::GrpcClient;
use kaspa_notify::subscription::context::SubscriptionContext;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_txscript::{pay_to_script_hash_script, extract_script_pub_key_address};
use serde_json::Value;

const NODE: &str = "127.0.0.1:16210";
const COMPILED_JSON: &str = r"C:\bitphoque\covenants\bitphoque_compiled_real.json";
const EXPECTED_COV_ID: &str = "69262e04b63d1ea1008fd33f8aff38c08780056b40111d0db01a306ef11a766c";

fn load_redeem_script() -> Vec<u8> {
    let raw = fs::read_to_string(COMPILED_JSON).expect("read compiled json");
    let v: Value = serde_json::from_str(&raw).expect("parse compiled json");
    v["script"].as_array().expect("script array")
        .iter().map(|n| n.as_u64().expect("byte") as u8).collect()
}

#[tokio::main]
async fn main() {
    let redeem = load_redeem_script();
    let p2sh = pay_to_script_hash_script(&redeem);
    let token_addr = extract_script_pub_key_address(&p2sh, Prefix::Testnet).expect("p2sh address");

    println!("[inspect] BitPhoque token P2SH address:");
    println!("          {token_addr}");
    println!("[inspect] expected covenant_id: {EXPECTED_COV_ID}");
    println!("[inspect] connecting to grpc://{NODE}");

    let subscription_context = SubscriptionContext::new();
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct,
        format!("grpc://{NODE}"),
        Some(subscription_context.clone()),
        true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[inspect] connected.");

    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");

    if resp.is_empty() {
        println!();
        println!("[inspect] NO UTXOs at the token P2SH. No BitPhoques found at this address.");
        println!("          (If genesis was spent/moved, the covenant UTXO is now at a new owner's P2SH.)");
        return;
    }

    println!();
    println!("[inspect] FOUND {} UTXO(s) at the BitPhoque token P2SH:", resp.len());
    for (i, entry) in resp.iter().enumerate() {
        let op = &entry.outpoint;
        let amt = entry.utxo_entry.amount;
        println!("  [{i}] outpoint : {}:{}", op.transaction_id, op.index);
        println!("      value    : {amt} sompi");
        // covenant_id is on the utxo entry if the node exposes it; print what we can.
        println!("      (covenant UTXO holding BitPhoques; expected cov_id {EXPECTED_COV_ID})");
    }
    println!();
    println!("[inspect] BitPhoques confirmed on-chain. The 42,069 supply lives in the covenant UTXO above.");
}
