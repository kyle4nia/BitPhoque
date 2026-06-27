// BitPhoque — CLAIM harness (TN10 practice). STEP 3 of the on-chain sale.
//
// A BUYER claims the for-sale v2 covenant UTXO: pays >= price to the seller and receives the
// phoques. The covenant input is UNSIGNED (payment-authorized); a second plain input (the buyer's
// funds) pays the seller + fee and is buyer-signed. This is the unsigned-spend path ON-CHAIN.
//
// Tx shape:
//   input 0: for-sale covenant UTXO (cad1220f...:0). UNSIGNED. ComputeBudget mass. witness -> payment output.
//   input 1: buyer's plain P2PK funding UTXO. buyer-SIGNED. SigopCount(1) mass.
//   output 0: phoques -> BUYER (new covenant UTXO, forSale=false). bound, authorizing_input 0.
//   output 1: payment -> SELLER P2PK (>= price). plain.
//   output 2: change -> buyer P2PK. plain.
//
// dry-run by default; --go to broadcast. LOCAL VM VERIFY (input 0) gates --go.
// TN10 PRACTICE ONLY.

use std::fs;

use kaspa_addresses::{Address, Prefix, Version};
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

const NODE: &str = "127.0.0.1:16210";
const SELLER_KEY_FILE: &str = r"C:\bitphoque\bitphoque_v2_owner.key"; // seller = current owner of the for-sale UTXO
const BUYER_KEY_FILE: &str = r"C:\bitphoque\bitphoque_buyer.key";     // buyer funds the payment, receives phoques
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v2.sil";
const COVENANT_ID_HEX: &str = "9a30cef719c6ac08eaf94f701dba43136f3a6837b4d24c419610507250367bf3";

const SUPPLY: i64 = 42_069;
const PRICE: u64 = 500_000;       // must match the listed price
const PAYMENT: u64 = 100_000_000;     // buyer pays exactly the price (>= PRICE required)
const FEE: u64 = 700_000;         // claim reveals 1969B script + 2 inputs -> higher floor; guess, node corrects
const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;
const COVENANT_SCRIPT_UNITS: u64 = 180_000;

fn ctor_args(owner: Vec<u8>, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner), Expr::int(SUPPLY), Expr::byte(0), Expr::bool(false),
        Expr::bool(for_sale), Expr::int(price), Expr::int(MAX_COV_INS), Expr::int(MAX_COV_OUTS),
    ]
}

fn new_state(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Expr<'static> {
    struct_object(vec![
        ("ownerIdentifier", Expr::bytes(owner)),
        ("identifierType", Expr::byte(0)),
        ("amount", Expr::int(amount)),
        ("isMinter", Expr::bool(false)),
        ("forSale", Expr::bool(for_sale)),
        ("price", Expr::int(price)),
    ])
}

fn load_key(path: &str) -> Keypair {
    let hex = fs::read_to_string(path).expect("read key file");
    let hex = hex.trim();
    let mut sk = [0u8; 32];
    faster_hex::hex_decode(hex.as_bytes(), &mut sk).expect("bad hex key");
    Keypair::from_seckey_slice(secp256k1::SECP256K1, &sk).expect("bad key")
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

#[tokio::main]
async fn main() {
    let go = std::env::args().any(|a| a == "--go");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let cov_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let cov_mass = TxInputMass::ComputeBudget(cov_budget);
    let plain_mass = TxInputMass::ComputeBudget(11.into());

    let seller = load_key(SELLER_KEY_FILE);
    let seller_pk = seller.x_only_public_key().0.serialize().to_vec();
    let buyer = load_key(BUYER_KEY_FILE);
    let buyer_pk = buyer.x_only_public_key().0.serialize().to_vec();
    let seller_addr = Address::new(Prefix::Testnet, Version::PubKey, &seller_pk);
    let buyer_addr = Address::new(Prefix::Testnet, Version::PubKey, &buyer_pk);
    println!("[claim] seller (payout) addr: {seller_addr}");
    println!("[claim] buyer addr: {buyer_addr}");

    // The for-sale input contract = seller-owned, forSale=true, price=PRICE.
    let forsale_compiled = compile_contract(&source, &ctor_args(seller_pk.clone(), true, PRICE as i64), CompileOptions::default()).expect("compile for-sale");
    let forsale_p2sh = pay_to_script_hash_script(&forsale_compiled.script);
    // Output token contract = BUYER-owned, forSale=false (claim resets the flag).
    let buyer_token_compiled = compile_contract(&source, &ctor_args(buyer_pk.clone(), false, 0), CompileOptions::default()).expect("compile buyer token");
    let buyer_token_p2sh = pay_to_script_hash_script(&buyer_token_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[claim] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[claim] connected.");

    // Find the for-sale covenant UTXO.
    let forsale_addr = extract_script_pub_key_address(&forsale_p2sh, Prefix::Testnet).expect("fs p2sh address");
    let fs_resp = rpc.get_utxos_by_addresses(vec![forsale_addr]).await.expect("get_utxos fs");
    if fs_resp.is_empty() { println!("[claim] no for-sale covenant UTXO found. aborting."); return; }
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
    if b_value <= PAYMENT + FEE { println!("[claim] buyer funds {b_value} <= payment+fee {}. aborting.", PAYMENT + FEE); return; }

    let cov_id = covenant_id();
    let fs_utxo = UtxoEntry::new(fs_value, forsale_p2sh.clone(), 0, false, Some(cov_id));
    let b_utxo = UtxoEntry::new(b_value, buyer_spk.clone(), 0, false, None);

    // Outputs: token->buyer (covenant), payment->seller (plain), change->buyer (plain).
    let token_out = TransactionOutput {
        value: fs_value, // the covenant carries its full value to the new token UTXO
        script_public_key: buyer_token_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };
    let payment_out = TransactionOutput { value: PAYMENT, script_public_key: pay_to_address_script(&seller_addr), covenant: None };
    let change_value = b_value - PAYMENT - FEE;
    let change_out = TransactionOutput { value: change_value, script_public_key: buyer_spk.clone(), covenant: None };
    let outputs = vec![token_out, payment_out, change_out];

    // Build unsigned skeleton (both inputs empty sigscript) to compute buyer's sighash on input 1.
    let in0_unsigned = TransactionInput { previous_outpoint: fs_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let in1_unsigned = TransactionInput { previous_outpoint: b_outpoint, signature_script: vec![], sequence: 0, mass: plain_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0_unsigned, in1_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);

    // Sign INPUT 1 (buyer's plain funding) over the tx.
    let mtx = MutableTransaction::with_entries(unsigned, vec![fs_utxo.clone(), b_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let b_sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 1, SIG_HASH_ALL, &reused);
    let b_msg = secp256k1::Message::from_digest_slice(b_sig_hash.as_bytes().as_slice()).expect("buyer sighash");
    let b_sig = buyer.sign_schnorr(b_msg);
    let mut b_signature = b_sig.as_ref().to_vec();
    b_signature.push(SIG_HASH_ALL.to_u8());
    // Buyer's plain P2PK sigscript: push(sig).
    let in1_sigscript = ScriptBuilder::new().add_data(&b_signature).expect("push buyer sig").drain();

    // INPUT 0 covenant leader sigscript: new-state (buyer-owned, forSale=false), dummy sig, witness->payment output (index 1).
    let new_states: Expr = vec![new_state(buyer_pk.clone(), SUPPLY, false, 0)].into();
    let sigs: Expr = vec![Expr::bytes(vec![0u8; 65])].into(); // dummy; for-sale branch is payment-authorized
    let witnesses: Expr = Expr::bytes(vec![1u8]);             // payment output index
    let mut in0_sigscript = forsale_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&forsale_compiled.script).expect("push redeem").drain();
    in0_sigscript.extend_from_slice(&redeem_push);

    let in0 = TransactionInput { previous_outpoint: fs_outpoint, signature_script: in0_sigscript, sequence: 0, mass: cov_mass };
    let in1 = TransactionInput { previous_outpoint: b_outpoint, signature_script: in1_sigscript, sequence: 0, mass: plain_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0, in1], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[claim] claim tx id: {}", tx.id());
    println!("[claim] buyer pays {PAYMENT} to seller, receives {SUPPLY} phoques; change {change_value}; fee {FEE}");

    // LOCAL VM VERIFY the COVENANT input (input 0) — the payment-authorized spend.
    println!("[claim] local VM verify (covenant input 0)...");
    match local_vm_verify(&tx, vec![fs_utxo.clone(), b_utxo.clone()], 0) {
        Ok(()) => println!("[claim] LOCAL VM VERIFY: PASS — the covenant accepts this claim."),
        Err(e) => { println!("[claim] LOCAL VM VERIFY: FAIL — {e:?}"); println!("[claim] aborting."); return; }
    }

    if !go {
        println!();
        println!("[claim] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[claim] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[claim] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[claim] REJECTED: {e}"),
    }
}
