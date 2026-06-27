// BitPhoque — LISTING harness (TN10 practice). STEP 2 of the on-chain sale.
//
// Spends the v2 token's not-for-sale covenant UTXO and creates ONE new covenant UTXO,
// SAME owner, flipped to forSale=true at PRICE. This is the owner-SIGNED listing path.
// (Listing does not change the owner — it just marks the UTXO for sale.)
//
// Adapted from bitphoque_transfer.rs (proven). Differences:
//  - v2 contract / v2 covenant_id / v2 owner key
//  - 8-arg ctor (adds forSale, price); 6-field new_state
//  - output owner == input owner; output forSale=true, price=PRICE
//  - input is NOT-for-sale -> signature path -> sign with the v2 owner key
//  - bigger script (1969B) -> higher script-unit cost + fee; budget via checked_covering_script_units
//
// dry-run by default; --go to broadcast. LOCAL VM VERIFY gates --go.
// TN10 PRACTICE ONLY.

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

const NODE: &str = "127.0.0.1:16210";
const KEY_FILE: &str = r"C:\bitphoque\bitphoque_v2_owner.key";
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v2.sil";
const COVENANT_ID_HEX: &str = "9a30cef719c6ac08eaf94f701dba43136f3a6837b4d24c419610507250367bf3";

const SUPPLY: i64 = 42_069;
const FEE: u64 = 500_000;
const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;
const PRICE: i64 = 500_000; // sompi the buyer must pay to claim (set by this listing)
// Generous upper bound for the v2 (1969B) sale spend; checked_covering_script_units derives the
// budget from this. If the node rejects with a higher used=N, bump this to N.
const COVENANT_SCRIPT_UNITS: u64 = 180_000;

// 8-arg ctor: owner, amount, idType=0, isMinter=false, forSale, price, maxIns, maxOuts.
fn ctor_args(owner: Vec<u8>, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner),
        Expr::int(SUPPLY),
        Expr::byte(0),
        Expr::bool(false),
        Expr::bool(for_sale),
        Expr::int(price),
        Expr::int(MAX_COV_INS),
        Expr::int(MAX_COV_OUTS),
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

fn load_keypair() -> Keypair {
    let hex = fs::read_to_string(KEY_FILE).expect("read key file");
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
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);
    println!("[list] input compute budget: {:?} (covers {} script units)", input_budget, COVENANT_SCRIPT_UNITS);

    let owner = load_keypair();
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    println!("[list] owner pubkey: {}", faster_hex::hex_string(&owner_pk));
    let owner_addr = Address::new(Prefix::Testnet, Version::PubKey, &owner_pk);
    println!("[list] owner P2PK addr (the payout address a buyer must pay): {owner_addr}");

    // Input contract: the v2 token's current state = NOT for sale.
    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);

    // Output contract: SAME owner, FOR SALE at PRICE.
    let out_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), true, PRICE), CompileOptions::default()).expect("compile output");
    let out_p2sh = pay_to_script_hash_script(&out_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[list] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[list] connected.");

    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() {
        println!("[list] no covenant UTXO at the v2 token P2SH (not-for-sale state). aborting.");
        return;
    }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[list] spending v2 cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));

    let out_value = in_value - FEE;
    let output = TransactionOutput {
        value: out_value,
        script_public_key: out_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };

    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], vec![output.clone()], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);

    // Sign input 0 with the v2 owner key (input is not-for-sale -> signature path).
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = owner.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    // Output state: same owner, forSale=true, price=PRICE.
    let new_states: Expr = vec![new_state(owner_pk.clone(), SUPPLY, true, PRICE)].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]); // not used on the signed branch
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], vec![output], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[list] listing tx id: {}", tx.id());
    println!("[list] listing {SUPPLY} phoques FOR SALE at price {PRICE} sompi; output value {out_value} (fee {FEE})");

    println!("[list] local VM verify (TxScriptEngine, covenants_enabled)...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[list] LOCAL VM VERIFY: PASS — the covenant accepts this listing."),
        Err(e) => { println!("[list] LOCAL VM VERIFY: FAIL — {e:?}"); println!("[list] aborting."); return; }
    }

    if !go {
        println!();
        println!("[list] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[list] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[list] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[list] REJECTED: {e}"),
    }
}
