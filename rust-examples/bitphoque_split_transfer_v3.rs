// BitPhoque — TRANSFER-SPLIT harness, v3 (TN10 practice). Generalizes the proven self-split.
//
// Splits ONE not-for-sale v3 covenant UTXO into TWO not-for-sale covenant UTXOs whose owners
// may DIFFER from each other and from the signer. The input owner (signer) authorizes the spend;
// conservation is about AMOUNTS, ownership is set per-output freely. Used to seed a realistic
// multi-holder distribution (e.g. keep a remainder, hand a parcel to a new holder).
//
// v3 contract path (verified against bitphoque_sale_v3.sil — same as the self-split, owner of the
// OUTPUTS is not constrained to equal the input owner):
//   checkAuth      -> input not-for-sale -> require(checkSig(SIGNER))          [signer owns the input]
//   checkAmounts   -> totalIn == out0_amount + out1_amount                     [conservation]
//   checkMinting   -> both outputs isMinter=false                             [non-minter]
//   checkSaleState -> no input for-sale -> for-sale-output restriction skipped [plain transfer]
//
// CLI: <in_amount> <signer_keyfile> <out0_amount> <out0_pubkey_hex> <out1_amount> <out1_pubkey_hex> [--go]
//   e.g. 21034 C:\bitphoque\bitphoque_buyer1.key \
//        1000  5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161 \   (-> buyer2)
//        20034 <buyer1_pubkey_hex>                                                   (-> buyer1 keeps)
//
// NOTE: in_amount selects WHICH input UTXO (amount is a ctor arg -> the input sits at a specific
// P2SH derived from signer_pubkey + in_amount). The signer must currently own that UTXO.
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

const NODE: &str = "192.168.0.206:16210";
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";

const FEE: u64 = 700_000;
const MAX_COV_INS: i64 = 3;
const MAX_COV_OUTS: i64 = 3;
const COVENANT_SCRIPT_UNITS: u64 = 240_000;

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

fn decode_pubkey(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    let mut pk = [0u8; 32];
    faster_hex::hex_decode(hex.as_bytes(), &mut pk).expect("bad pubkey hex (want 32-byte x-only)");
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

#[tokio::main]
async fn main() {
    // CLI: <in_amount> <signer_keyfile> <out0_amount> <out0_pubkey> <out1_amount> <out1_pubkey> [--go]
    let raw: Vec<String> = std::env::args().collect();
    let go = raw.iter().any(|a| a == "--go");
    let pos: Vec<&String> = raw.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.len() < 6 {
        eprintln!("usage: bitphoque_split_transfer_v3 <in_amount> <signer_keyfile> <out0_amount> <out0_pubkey_hex> <out1_amount> <out1_pubkey_hex> [--go]");
        std::process::exit(2);
    }
    let in_amount: i64 = pos[0].parse().expect("in_amount must be an integer");
    let signer_key_file = pos[1].as_str();
    let out0_amount: i64 = pos[2].parse().expect("out0_amount must be an integer");
    let out0_pk = decode_pubkey(pos[3]);
    let out1_amount: i64 = pos[4].parse().expect("out1_amount must be an integer");
    let out1_pk = decode_pubkey(pos[5]);

    // sanity: amounts must conserve
    assert_eq!(out0_amount + out1_amount, in_amount, "split amounts must sum to in_amount");
    println!("[split] in_amount {in_amount} -> out0 {out0_amount} + out1 {out1_amount}");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);
    println!("[split] input compute budget: {:?} (covers {} script units)", input_budget, COVENANT_SCRIPT_UNITS);

    let signer = load_keypair(signer_key_file);
    let signer_pk = signer.x_only_public_key().0.serialize().to_vec();
    println!("[split] signer (input owner) pubkey: {}", faster_hex::hex_string(&signer_pk));
    let signer_addr = Address::new(Prefix::Testnet, Version::PubKey, &signer_pk);
    println!("[split] signer P2PK addr: {signer_addr}");
    println!("[split] out0 owner pubkey: {}", faster_hex::hex_string(&out0_pk));
    println!("[split] out1 owner pubkey: {}", faster_hex::hex_string(&out1_pk));

    // Input contract: signer-owned, NOT for sale, at in_amount.
    let in_compiled = compile_contract(&source, &ctor_args(signer_pk.clone(), in_amount, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);

    // Output contracts: out0 owner / out1 owner, NOT for sale, split amounts.
    let out0_compiled = compile_contract(&source, &ctor_args(out0_pk.clone(), out0_amount, false, 0), CompileOptions::default()).expect("compile out0");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);
    let out1_compiled = compile_contract(&source, &ctor_args(out1_pk.clone(), out1_amount, false, 0), CompileOptions::default()).expect("compile out1");
    let out1_p2sh = pay_to_script_hash_script(&out1_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[split] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[split] connected.");

    // Find the input covenant UTXO (signer-owned, in_amount, not-for-sale P2SH).
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    println!("[split] looking for not-for-sale cov UTXO ({in_amount} phoques) at P2SH {token_addr}");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() {
        println!("[split] no covenant UTXO at this P2SH (signer/in_amount/caps must match). aborting.");
        return;
    }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[split] spending cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);

    if in_value <= FEE {
        println!("[split] covenant value {in_value} <= fee {FEE}. aborting.");
        return;
    }

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, in_p2sh.clone(), 0, false, Some(cov_id));

    // Split the covenant's TKAS value ~evenly; odd sompi -> output 0.
    let spendable = in_value - FEE;
    let val1 = spendable / 2;
    let val0 = spendable - val1; // val0 gets the odd sompi
    println!("[split] covenant value split: out0 {val0} sompi, out1 {val1} sompi (fee {FEE})");

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

    // Sign input 0 with the signer key (input is not-for-sale -> signature path).
    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = signer.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    // Output states: out0 owner / out1 owner, not-for-sale, split amounts (2-element array).
    let new_states: Expr = vec![
        new_state(out0_pk.clone(), out0_amount, false, 0),
        new_state(out1_pk.clone(), out1_amount, false, 0),
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

    println!("[split] split tx id: {}", tx.id());
    println!("[split] splitting {in_amount} phoques -> {out0_amount} (out0 owner) + {out1_amount} (out1 owner)");

    println!("[split] local VM verify (TxScriptEngine, covenants_enabled)...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[split] LOCAL VM VERIFY: PASS — the covenant accepts this transfer-split."),
        Err(e) => { println!("[split] LOCAL VM VERIFY: FAIL — {e:?}"); println!("[split] aborting."); return; }
    }

    if !go {
        println!();
        println!("[split] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[split] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[split] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[split] REJECTED: {e}"),
    }
}
