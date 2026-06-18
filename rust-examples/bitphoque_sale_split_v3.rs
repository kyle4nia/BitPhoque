// BitPhoque — SPLIT harness, v3 (TN10 practice). PREREQUISITE for the 2x2 on-chain sale.
//
// The v3 genesis minted all 42069 phoques into ONE covenant UTXO. A 2x2 list+claim needs
// TWO for-sale UTXOs, so we first SPLIT the genesis UTXO into two owner-owned, not-for-sale
// covenant UTXOs (21034 + 21035 = 42069). This is a plain owner-SIGNED transfer:
//   1 covenant input (the genesis UTXO, not-for-sale -> signature path)
//   2 covenant outputs (same owner, not-for-sale, amounts splitting the supply)
//
// v3 contract path through a split (verified against bitphoque_sale_v3.sil):
//   checkAuth      -> input not-for-sale -> require(checkSig(owner))           [owner-signed]
//   checkAmounts   -> totalIn(42069) == totalOut(21034+21035)                  [conservation]
//   checkMinting   -> both outputs isMinter=false                             [non-minter]
//   checkSaleState -> no input for-sale -> for-sale-output restriction skipped [plain split]
//
// Adapted from bitphoque_sale_list.rs (proven owner-signed covenant spend). Differences:
//   - v3 source / v3 covenant_id / 8-arg ctor (forSale=false, price=0 on both outputs)
//   - TWO covenant outputs instead of one; new_states is a 2-element array
//   - binding authorizes input 0 for BOTH outputs
//   - covenant TKAS value split ~evenly across the two outputs (odd sompi -> output 0)
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

const NODE: &str = "192.168.0.206:16210";
const KEY_FILE: &str = r"C:\bitphoque\bitphoque_v2_owner.key"; // owner of the v3 genesis UTXO
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";

const SUPPLY: i64 = 42_069;
const AMOUNT_0: i64 = 21_034; // output 0 phoques
const AMOUNT_1: i64 = 21_035; // output 1 phoques  (21034 + 21035 = 42069)
const FEE: u64 = 700_000;
const MAX_COV_INS: i64 = 3;
const MAX_COV_OUTS: i64 = 3;
// Generous upper bound for the v3 (2793B) sale spend; checked_covering_script_units derives the
// budget. If the node rejects with a higher used=N, bump this to N.
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

    // sanity: amounts must conserve
    assert_eq!(AMOUNT_0 + AMOUNT_1, SUPPLY, "split amounts must sum to supply");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let input_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let input_mass = TxInputMass::ComputeBudget(input_budget);
    println!("[split] input compute budget: {:?} (covers {} script units)", input_budget, COVENANT_SCRIPT_UNITS);

    let owner = load_keypair();
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    println!("[split] owner pubkey: {}", faster_hex::hex_string(&owner_pk));
    let owner_addr = Address::new(Prefix::Testnet, Version::PubKey, &owner_pk);
    println!("[split] owner P2PK addr: {owner_addr}");

    // Input contract: the genesis state = NOT for sale, full supply.
    let in_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), SUPPLY, false, 0), CompileOptions::default()).expect("compile input");
    let in_p2sh = pay_to_script_hash_script(&in_compiled.script);

    // Output contracts: SAME owner, NOT for sale, split amounts.
    let out0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), AMOUNT_0, false, 0), CompileOptions::default()).expect("compile out0");
    let out0_p2sh = pay_to_script_hash_script(&out0_compiled.script);
    let out1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), AMOUNT_1, false, 0), CompileOptions::default()).expect("compile out1");
    let out1_p2sh = pay_to_script_hash_script(&out1_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[split] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[split] connected.");

    // Find the genesis covenant UTXO (full-supply, not-for-sale P2SH).
    let token_addr = extract_script_pub_key_address(&in_p2sh, Prefix::Testnet).expect("p2sh address");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() {
        println!("[split] no covenant UTXO at the v3 genesis P2SH (full-supply, not-for-sale). aborting.");
        return;
    }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[split] spending v3 genesis cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);

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

    // Sign input 0 with the owner key (input is not-for-sale -> signature path).
    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: input_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = owner.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    // Output states: same owner, not-for-sale, split amounts (2-element array).
    let new_states: Expr = vec![
        new_state(owner_pk.clone(), AMOUNT_0, false, 0),
        new_state(owner_pk.clone(), AMOUNT_1, false, 0),
    ].into();
    // One signature for the single covenant input.
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    // Witnesses unused on the signed branch, but the ABI expects one entry per input.
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = in_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: input_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[split] split tx id: {}", tx.id());
    println!("[split] splitting {SUPPLY} phoques -> {AMOUNT_0} + {AMOUNT_1} (same owner, not-for-sale)");

    println!("[split] local VM verify (TxScriptEngine, covenants_enabled)...");
    match local_vm_verify(&tx, vec![in_utxo.clone()], 0) {
        Ok(()) => println!("[split] LOCAL VM VERIFY: PASS — the covenant accepts this split."),
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
