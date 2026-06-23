// BitPhoque — NODE DIAGNOSTIC (read-only). No keys, no tx, no state writes. Stream-safe.
//
// Answers three questions about PsychoNode in ONE connection:
//   (1) Is the node up, connected, and SYNCED? (get_info + get_block_dag_info)
//   (2) Does a KNOWN-OLD covenant UTXO still resolve? (5000 for-sale parcel, 4b0c8187...:0,
//       inspect found it cleanly minutes ago) -> tests whether the address index serves old UTXOs.
//   (3) Does a KNOWN-NEW covenant UTXO resolve? (3000 for-sale parcel from 8dff6df4...) ->
//       tests whether recently-created UTXOs are indexed yet.
//
// Back-to-back on the SAME connection so old vs new is an apples-to-apples comparison:
//   old YES / new NO   -> pure address-index LAG on recent txs (wait + retry)
//   old NO  / new NO   -> index is behind/broken GLOBALLY (node health problem)
//   old YES / new YES  -> parcels are fine; earlier misses were transient
//
// Also times each RPC round-trip so a slow/loaded node shows up as latency.

use std::time::Instant;
use std::fs;

use kaspa_addresses::Prefix;
use kaspa_grpc_client::GrpcClient;
use kaspa_notify::subscription::context::SubscriptionContext;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_txscript::{pay_to_script_hash_script, extract_script_pub_key_address};
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, CompileOptions};

const NODE: &str = "192.168.0.104:16210";
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";

const MAX_COV_INS: i64 = 3;
const MAX_COV_OUTS: i64 = 3;

// The owner of the known parcels (buyer1).
const OWNER_HEX: &str = "8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936";

fn ctor_args(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner), Expr::int(amount), Expr::byte(0), Expr::bool(false),
        Expr::bool(for_sale), Expr::int(price), Expr::int(MAX_COV_INS), Expr::int(MAX_COV_OUTS),
    ]
}

fn decode_pubkey(hex: &str) -> Vec<u8> {
    let mut pk = [0u8; 32];
    faster_hex::hex_decode(hex.trim().as_bytes(), &mut pk).expect("bad pubkey hex");
    pk.to_vec()
}

async fn probe(rpc: &GrpcClient, source: &str, owner: &[u8], label: &str, amount: i64, for_sale: bool, price: i64) {
    let compiled = compile_contract(source, &ctor_args(owner.to_vec(), amount, for_sale, price), CompileOptions::default()).expect("compile");
    let p2sh = pay_to_script_hash_script(&compiled.script);
    let addr = extract_script_pub_key_address(&p2sh, Prefix::Testnet).expect("addr");
    let t0 = Instant::now();
    let resp = rpc.get_utxos_by_addresses(vec![addr.clone()]).await;
    let ms = t0.elapsed().as_millis();
    match resp {
        Ok(utxos) if !utxos.is_empty() => {
            let e = &utxos[0];
            let op = &e.outpoint;
            println!("  [{label}] FOUND ({ms} ms): {}:{}  value {} sompi", op.transaction_id, op.index, e.utxo_entry.amount);
        }
        Ok(_) => println!("  [{label}] NOT FOUND ({ms} ms) at {addr}"),
        Err(err) => println!("  [{label}] RPC ERROR ({ms} ms): {err}"),
    }
}

#[tokio::main]
async fn main() {
    println!("[diag] node {NODE}");
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let owner = decode_pubkey(OWNER_HEX);

    let subscription_context = SubscriptionContext::new();
    let t0 = Instant::now();
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[diag] connected in {} ms", t0.elapsed().as_millis());

    // (1) Node health.
    match rpc.get_info().await {
        Ok(info) => println!("[diag] get_info: synced={} utxo_indexed={} server_v={} mempool={}",
            info.is_synced, info.is_utxo_indexed, info.server_version, info.mempool_size),
        Err(e) => println!("[diag] get_info ERROR: {e}"),
    }
    match rpc.get_block_dag_info().await {
        Ok(d) => println!("[diag] dag: blocks={} headers={} tips={} virtual_daa={} sink={}",
            d.block_count, d.header_count, d.tip_hashes.len(), d.virtual_daa_score, d.sink),
        Err(e) => println!("[diag] get_block_dag_info ERROR: {e}"),
    }

    // (2) + (3) old vs new, same connection.
    println!("[diag] --- known-OLD parcel (5000 for-sale, from 4b0c8187...) ---");
    probe(&rpc, &source, &owner, "old 5000", 5000, true, 500_000).await;

    println!("[diag] --- known-NEW parcel (3000 for-sale, from 8dff6df4...) ---");
    probe(&rpc, &source, &owner, "new 3000", 3000, true, 500_000).await;

    println!("[diag] --- known-NEW parcel (17301 not-for-sale, from 8dff6df4...) ---");
    probe(&rpc, &source, &owner, "new 17301", 17301, false, 0).await;

    println!("[diag] --- spent INPUT parcel (20301 not-for-sale, from 4b0c8187...:1) ---");
    probe(&rpc, &source, &owner, "spent 20301", 20301, false, 0).await;

    println!("[diag] done. interpretation:");
    println!("       old FOUND + new NOT  -> address-index LAG on recent txs (wait + retry)");
    println!("       old NOT + new NOT    -> index behind/broken globally (node health)");
    println!("       new FOUND            -> parcels fine; earlier misses were transient");
}
