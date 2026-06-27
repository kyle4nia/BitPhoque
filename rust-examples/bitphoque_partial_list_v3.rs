// BitPhoque — PARTIAL-LISTING harness, v3 (TN10 practice). Lists PART of a balance in ONE tx.
//
// Takes ONE not-for-sale v3 covenant UTXO and, in a single OWNER-SIGNED tx, produces:
//   out0 = a FOR-SALE parcel (list_amount phoques at `price`)   <- the part put up for sale
//   out1 = a NOT-FOR-SALE change parcel (in_amount - list_amount)<- the part the owner keeps
// Both outputs owned by the SAME signer (you list YOUR phoques and keep the rest). Conservation is
// on AMOUNTS (out0 + out1 == in_amount). This collapses the old two-step "split, then list a whole
// parcel" into ONE tx, so a seller can list an arbitrary partial balance directly.
//
// Minimal diff from the proven self-split (bitphoque_split_transfer_v3.rs); the ONLY changes:
//   (1) out0 is for-sale at `price` (was not-for-sale); out1 stays not-for-sale change.
//   (2) BOTH outputs owned by the signer (was two independent CLI pubkeys) -> owner implied by keyfile.
//   (3) TKAS value split is PROPORTIONAL to phoque amount (was even 50/50).
//
// v3 contract path (verified against bitphoque_sale_v3.sil — this is the LISTING direction):
//   checkAuth      -> input NOT for-sale -> require(checkSig(SIGNER)); witnesses untouched [owner-signed]
//   checkAmounts   -> totalIn == out0_amount + out1_amount                                 [conservation]
//   checkMinting   -> both outputs isMinter=false                                          [non-minter]
//   checkSaleState -> NO INPUT for-sale -> for-sale-OUTPUT restriction SKIPPED -> out0 may be for-sale
//                     (this is exactly the "LISTING (owner-signed, all inputs NOT for-sale):
//                      for-sale outputs ALLOWED" rule in the contract.)
//
// CLI: <in_amount> <owner_keyfile> <list_amount> <price> [--go]
//   e.g. 21034 C:\bitphoque\bitphoque_buyer1.key 5000 500000      (list 5000 @ 500000 sompi, keep 16034)
//
// NOTE: in_amount selects WHICH input UTXO (amount is a ctor arg -> the input sits at a specific P2SH
// derived from signer_pubkey + in_amount + not-for-sale). The signer must currently own that UTXO.
//
// dry-run by default; --go to broadcast. LOCAL VM VERIFY gates --go. TN10 PRACTICE ONLY.

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
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";

const FEE: u64 = 700_000;
const MAX_COV_INS: i64 = 3;
const MAX_COV_OUTS: i64 = 3;
const COVENANT_SCRIPT_UNITS: u64 = 240_000;
// Sanity floor: a covenant output below this is too small to be a useful/claimable parcel on TN10.
// Not a contract rule — a guard so a lopsided proportional split can't emit a near-dust for-sale UTXO.
const MIN_OUT_VALUE: u64 = 1_000_000;

// 8-arg ctor: owner, amount, idType=0, isMinter=false, forSale, price, maxIns, maxOuts.
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

fn load_keypair(path: &str) -> Keypair {
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
    // CLI: <in_amount> <owner_keyfile> <list_amount> <price> [--go]
    let raw: Vec<String> = std::env::args().collect();
    let go = raw.iter().any(|a| a == "--go");
    let pos: Vec<&String> = raw.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.len() < 4 {
        eprintln!("usage: bitphoque_partial_list_v3 <in_amount> <owner_keyfile> <list_amount> <price> [--go]");
        std::process::exit(2);
    }
    let in_amount: i64 = pos[0].parse().expect("in_amount must be an integer");
    let owner_key_file = pos[1].as_str();
    let list_amount: i64 = pos[2].parse().expect("list_amount must be an integer");
    let price: i64 = pos[3].parse().expect("price must be an integer");

    // sanity: list part of the balance, keep the rest; amounts must conserve.
    assert!(list_amount > 0, "list_amount must be > 0");
    assert!(list_amount < in_amount, "list_amount must be < in_amount (this lists PART of the balance; use the whole-parcel list harness to list everything)");
    assert!(price > 0, "price must be > 0");
    let keep_amount: i64 = in_amount - list_amount;
    println!("[plist] in_amount {in_amount} -> list {list_amount} @ price {price} (out0, FOR SALE) + keep {keep_amount} (out1, not-for-sale)");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);
    println!("[plist] input compute budget: {:?} (covers {} script units)", input_budget, COVENANT_SCRIPT_UNITS);

    let signer = load_keypair(owner_key_file);
    let owner_pk = signer.x_only_public_key().0.serialize().to_vec();
    println!("[plist] owner (signer, owns BOTH outputs) pubkey: {}", faster_hex::hex_string(&owner_pk));
    let owner_addr = Address::new(Prefix::Testnet, Version::PubKey, &owner_pk);
    println!("[plist] owner P2PK addr: {owner_addr}");

    // Input contract: owner-owned, NOT for sale, at in_amount.
    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), in_amount, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);

    // out0: owner, FOR SALE at `price`, list_amount.   out1: owner, NOT for sale, keep_amount.
    let out0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), list_amount, true, price), CompileOptions::default()).expect("compile out0 (for-sale)");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);
    let out1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), keep_amount, false, 0), CompileOptions::default()).expect("compile out1 (change)");
    let out1_p2sh = pay_to_script_hash_script(&out1_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[plist] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[plist] connected.");

    // Find the input covenant UTXO (owner-owned, in_amount, not-for-sale P2SH).
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[plist] looking for not-for-sale cov UTXO ({in_amount} phoques) at P2SH {token_addr}");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() {
        println!("[plist] no covenant UTXO at this P2SH (owner/in_amount/caps must match). aborting.");
        return;
    }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[plist] spending cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);

    if in_value <= FEE {
        println!("[plist] covenant value {in_value} <= fee {FEE}. aborting.");
        return;
    }

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));

    // Split the covenant's TKAS value PROPORTIONAL to phoque amount; out1 absorbs the remainder
    // so val0 + val1 == spendable EXACTLY (integer math, no rounding gap — the node enforces
    // inputs == outputs + fee, and the contract enforces phoque conservation, not value).
    let spendable = in_value - FEE;
    // u128 intermediate to avoid overflow on the multiply.
    let val0: u64 = ((spendable as u128 * list_amount as u128) / in_amount as u128) as u64;
    let val1: u64 = spendable - val0; // change/keep output absorbs the remainder
    println!("[plist] covenant value split (proportional): out0(for-sale) {val0} sompi, out1(keep) {val1} sompi (fee {FEE})");

    // Guard: neither output may fall below the sane floor (a near-dust for-sale parcel can't be claimed).
    if val0 < MIN_OUT_VALUE || val1 < MIN_OUT_VALUE {
        println!("[plist] one output value is below the {MIN_OUT_VALUE} sompi floor (out0 {val0}, out1 {val1}).");
        println!("[plist] the input UTXO is too small to split this way (or list_amount is too lopsided). aborting.");
        return;
    }

    let output0 = TransactionOutput {
        value: val0,
        script_public_key: out0_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };
    let output1 = TransactionOutput {
        value: val1,
        script_public_key: out1_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };
    let outputs = vec![output0, output1];

    // Sign input 0 with the owner key (input is not-for-sale -> signature path).
    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = signer.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    // Output states: out0 owner FOR-SALE @ price / out1 owner not-for-sale change (2-element array).
    let new_states: Expr = vec![
        new_state(owner_pk.clone(), list_amount, true, price),
        new_state(owner_pk.clone(), keep_amount, false, 0),
    ].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]); // unused on the signed branch; ABI expects one entry per input
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[plist] partial-list tx id: {}", tx.id());
    println!("[plist] listing {list_amount} phoques @ {price} (out0, FOR SALE) + keeping {keep_amount} (out1, not-for-sale) — same owner");

    println!("[plist] local VM verify (TxScriptEngine, covenants_enabled)...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[plist] LOCAL VM VERIFY: PASS — the covenant accepts this partial listing."),
        Err(e) => { println!("[plist] LOCAL VM VERIFY: FAIL — {e:?}"); println!("[plist] aborting."); return; }
    }

    if !go {
        println!();
        println!("[plist] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[plist] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[plist] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[plist] REJECTED: {e}"),
    }
}
