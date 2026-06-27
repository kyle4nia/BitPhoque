// BitPhoque — GENESIS ESTABLISHING tx, v3 sale contract (TN10 practice).
//
// Spends the plain P2PK funding UTXO (faucet-funded, key in bitphoque_owner.key)
// and creates the genesis BitPhoque v3 sale token covenant UTXO at the token P2SH
// address, binding it via populate_genesis_covenants so it carries the correct
// covenant_id. v3 fixes the F1 double-claim hole (strict-increasing payment-output
// indices for for-sale inputs). Genesis is not-for-sale (forSale=false, price=0).
//
// THE GENESIS LESSON: a covenant UTXO must be CREATED as a bound output of a signed
// tx — NOT funded directly at the P2SH (that yields covenant_id:None, unspendable).
//
// Redeem script is READ AT RUNTIME from bitphoque_sale_compiled_v3.json (no
// transcription). The 3x3 ctor args (maxCovIns=3, maxCovOuts=3, v2 seller pubkey as
// genesisPk, 42069, fixed, not-for-sale) are already baked into that compiled script.
// Signing key (the FUNDING signer) is READ from bitphoque_owner.key (gitignored).
//
// SAFETY: dry-run by default. Pass `--go` to broadcast.
// TN10 PRACTICE ONLY — in-process signing with a testnet key. The mainnet version
// (air-gapped detached signing) is specified in docs/bitphoque_mainnet_checklist.md.

use std::fs;

use kaspa_addresses::Address;
use kaspa_consensus_core::{
    constants::TX_VERSION_TOCCATA,
    sign::sign,
    subnets::SUBNETWORK_ID_NATIVE,
    tx::{
        GenesisCovenantGroup, MutableTransaction, Transaction, TransactionInput,
        TransactionOutpoint, TransactionOutput, TxInputMass, UtxoEntry,
    },
};
use kaspa_grpc_client::GrpcClient;
use kaspa_notify::subscription::context::SubscriptionContext;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_txscript::pay_to_script_hash_script;
use secp256k1::Keypair;
use serde_json::Value;

const NODE: &str = "127.0.0.1:16210";
// Plain P2PK funding address we control (faucet-funded). Key in the file below.
const FUNDING_ADDR: &str =
    "kaspatest:qrwzyuc8lkvdsxgww53n7rsjetsqhh6p9hmy8vrvqpdun5r7ysevufmglhgl4";
const KEY_FILE: &str = r"C:\bitphoque\bitphoque_owner.key";
const COMPILED_JSON: &str = r"C:\bitphoque\covenants\bitphoque_sale_compiled_v3.json";

const FEE: u64 = 200_000;

// Read the redeem script (array of byte ints) out of the compiled-contract JSON.
fn load_redeem_script() -> Vec<u8> {
    let raw = fs::read_to_string(COMPILED_JSON).expect("read compiled json");
    let v: Value = serde_json::from_str(&raw).expect("parse compiled json");
    let arr = v["script"].as_array().expect("script field is array");
    arr.iter()
        .map(|n| n.as_u64().expect("script byte is int") as u8)
        .collect()
}

// Read + trim the hex privkey from the gitignored key file.
fn load_keypair() -> Keypair {
    let hex = fs::read_to_string(KEY_FILE).expect("read key file");
    let hex = hex.trim();
    let mut sk = [0u8; 32];
    faster_hex::hex_decode(hex.as_bytes(), &mut sk).expect("bad hex key");
    Keypair::from_seckey_slice(secp256k1::SECP256K1, &sk).expect("bad key")
}

#[tokio::main]
async fn main() {
    let go = std::env::args().any(|a| a == "--go");

    let redeem = load_redeem_script();
    println!("[genesis-v3] redeem script: {} bytes (read from compiled json)", redeem.len());

    let keypair = load_keypair();
    println!("[genesis-v3] signing key loaded from {KEY_FILE}");

    let subscription_context = SubscriptionContext::new();
    println!("[genesis-v3] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct,
        format!("grpc://{NODE}"),
        Some(subscription_context.clone()),
        true,
        None,
        false,
        Some(500_000),
        Default::default(),
    )
    .await
    .expect("connect failed");
    println!("[genesis-v3] connected.");

    // Fetch the funding UTXO.
    let addr = Address::try_from(FUNDING_ADDR).expect("bad funding addr");
    let resp = rpc.get_utxos_by_addresses(vec![addr]).await.expect("get_utxos failed");
    if resp.is_empty() {
        println!("[genesis-v3] no UTXO on funding address. aborting.");
        return;
    }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_amount = entry.utxo_entry.amount;
    let in_utxo = UtxoEntry::from(entry.utxo_entry.clone());
    println!(
        "[genesis-v3] funding input: {}:{}  amount {} sompi",
        outpoint.transaction_id, outpoint.index, in_amount
    );

    if in_amount <= FEE {
        println!("[genesis-v3] funding amount {in_amount} <= fee {FEE}. aborting.");
        return;
    }

    // Plain signed input (no covenant). SigopCount(1) = standard single-sig.
    let input = TransactionInput {
        previous_outpoint: outpoint,
        signature_script: vec![],
        sequence: 0,
        mass: TxInputMass::SigopCount(1.into()),
    };

    // Genesis covenant output: pays to the BitPhoque v3 token P2SH, carries the value.
    let covenant_spk = pay_to_script_hash_script(&redeem);
    let out_value = in_amount - FEE;
    let output = TransactionOutput {
        value: out_value,
        script_public_key: covenant_spk,
        covenant: None,
    };

    // Build unsigned tx (Toccata version so covenant bindings are allowed).
    let unsigned = Transaction::new_non_finalized(
        TX_VERSION_TOCCATA,
        vec![input],
        vec![output],
        0,
        SUBNETWORK_ID_NATIVE,
        0,
        vec![],
    );
    let mut mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo]);

    // Bind the genesis covenant: input 0 authorizes output 0.
    // Derives the covenant_id from (input outpoint, [output 0]) and sets it.
    mtx.tx
        .populate_genesis_covenants(&[GenesisCovenantGroup::new(0, vec![0])])
        .expect("populate_genesis_covenants failed");

    let bound_cid = mtx.tx.outputs[0].covenant.as_ref().map(|c| c.covenant_id);
    println!("[genesis-v3] genesis covenant_id bound to output: {bound_cid:?}");
    println!("[genesis-v3] covenant output value: {out_value} sompi (fee {FEE})");

    // Sign the plain input with our key.
    let signed = sign(mtx, keypair);
    let mut tx = signed.tx;
    tx.finalize();
    println!("[genesis-v3] built+signed tx id: {}", tx.id());

    if !go {
        println!();
        println!("[genesis-v3] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[genesis-v3] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[genesis-v3] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[genesis-v3] REJECTED: {e}"),
    }
}
