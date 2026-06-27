// BitPhoque — COMBINED 2x2 CLAIM harness v3 (TN10 practice). THE ON-CHAIN F1-FIX PROOF.
//
// ONE tx atomically claims TWO for-sale covenant UTXOs from the SAME seller, with each for-sale
// input naming a DISTINCT, STRICTLY-INCREASING payment output. This is the exact path that
// exercises v3's lastSaleWitness accumulator (seeded at -1; each for-sale witness must exceed the
// previous) — the fix for the F1 double-claim hole, where v2 let two for-sale inputs from one
// seller name the SAME payment output (two parcels for one payment). Same-seller is the strongest
// F1 test: cross-seller was already caught by the wrong-payee check (that was F2); same-seller is
// the hole, so both inputs paying the SAME seller means only the strict-increasing rule forces two
// outputs. Proven at the VM (2x2 and 3x3 reject-tests); this broadcasts it on a real node.
//
// Two buyers, each funding their OWN payment (true atomic multi-party settlement — nobody can be
// rugged; the whole tx commits or none of it does).
//
// Tx shape:
//   input 0: for-sale cov UTXO A (seller buyer1). UNSIGNED, payment-authorized. witness -> output 2.
//   input 1: for-sale cov UTXO B (seller buyer1). UNSIGNED, payment-authorized. witness -> output 3.
//   input 2: buyer A plain funding. buyer-A-SIGNED.
//   input 3: buyer B plain funding. buyer-B-SIGNED.
//   output 0: phoques A -> buyer A (covenant, authorizing_input 0)
//   output 1: phoques B -> buyer B (covenant, authorizing_input 1)
//   output 2: payment -> seller (from buyer A, >= price)   [witness for input 0]
//   output 3: payment -> seller (from buyer B, >= price)   [witness for input 1]  (3 > 2: strict-inc)
//   output 4: change -> buyer A
//   output 5: change -> buyer B
//
// Hardcoded for THIS proof's specific UTXOs/parties (it is a one-shot demonstration, not the
// general CLI — that comes later). Edit the consts to re-run with different inputs.
//
// dry-run by default; --go to broadcast. LOCAL VM VERIFY (both covenant inputs) gates --go.
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
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";

// --- This proof's specific parties + listings ---
const SELLER_PK_HEX: &str = "8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936"; // buyer1 = seller of both
const BUYER_A_KEY_FILE: &str = r"C:\bitphoque\bitphoque_buyer2.key"; // claims listing A (1 phoque)
const BUYER_B_KEY_FILE: &str = r"C:\bitphoque\bitphoque_buyer3.key"; // claims listing B (5267 phoques)
const AMOUNT_A: i64 = 1;       // listing A phoques
const AMOUNT_B: i64 = 5267;    // listing B phoques
const PRICE: i64 = 500_000;    // both listed at this price

const PAYMENT: u64 = 100_000_000; // each buyer pays 1 TKAS (>> price AND >> KIP-9 dust floor)
const FEE: u64 = 1_500_000;       // 4-input / 6-output / TWO script-reveal tx is the heaviest yet;
                                  // start high, the node corrects with the exact required figure.
const MAX_COV_INS: i64 = 3;       // MUST match genesis/split/listing (baked into P2SH)
const MAX_COV_OUTS: i64 = 3;
const COVENANT_SCRIPT_UNITS: u64 = 300_000;

fn ctor_args(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner), Expr::int(amount), Expr::byte(0), Expr::bool(false),
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

fn decode_pubkey(hex: &str) -> Vec<u8> {
    let mut pk = [0u8; 32];
    faster_hex::hex_decode(hex.trim().as_bytes(), &mut pk).expect("bad seller pubkey hex");
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
    let go = std::env::args().any(|a| a == "--go");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let cov_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let cov_mass = TxInputMass::ComputeBudget(cov_budget);
    let plain_mass = TxInputMass::ComputeBudget(11.into());

    let seller_pk = decode_pubkey(SELLER_PK_HEX);
    let seller_addr = Address::new(Prefix::Testnet, Version::PubKey, &seller_pk);
    let buyer_a = load_key(BUYER_A_KEY_FILE);
    let buyer_a_pk = buyer_a.x_only_public_key().0.serialize().to_vec();
    let buyer_a_addr = Address::new(Prefix::Testnet, Version::PubKey, &buyer_a_pk);
    let buyer_b = load_key(BUYER_B_KEY_FILE);
    let buyer_b_pk = buyer_b.x_only_public_key().0.serialize().to_vec();
    let buyer_b_addr = Address::new(Prefix::Testnet, Version::PubKey, &buyer_b_pk);
    println!("[2x2] seller (payout) addr: {seller_addr}");
    println!("[2x2] buyer A addr: {buyer_a_addr}  claims {AMOUNT_A} phoque(s)");
    println!("[2x2] buyer B addr: {buyer_b_addr}  claims {AMOUNT_B} phoque(s)");

    // For-sale input contracts: seller-owned, forSale=true, price, at each amount.
    let fsA_compiled = compile_contract(&source, &ctor_args(seller_pk.clone(), AMOUNT_A, true, PRICE), CompileOptions::default()).expect("compile fsA");
    let fsA_p2sh = pay_to_script_hash_script(&fsA_compiled.script);
    let fsB_compiled = compile_contract(&source, &ctor_args(seller_pk.clone(), AMOUNT_B, true, PRICE), CompileOptions::default()).expect("compile fsB");
    let fsB_p2sh = pay_to_script_hash_script(&fsB_compiled.script);
    // Output token contracts: buyer-owned, forSale=false, same amounts.
    let tokA_compiled = compile_contract(&source, &ctor_args(buyer_a_pk.clone(), AMOUNT_A, false, 0), CompileOptions::default()).expect("compile tokA");
    let tokA_p2sh = pay_to_script_hash_script(&tokA_compiled.script);
    let tokB_compiled = compile_contract(&source, &ctor_args(buyer_b_pk.clone(), AMOUNT_B, false, 0), CompileOptions::default()).expect("compile tokB");
    let tokB_p2sh = pay_to_script_hash_script(&tokB_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[2x2] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[2x2] connected.");

    // Find the two for-sale covenant UTXOs.
    let fsA_addr = extract_script_pub_key_address(&fsA_p2sh, Prefix::Testnet).expect("fsA addr");
    let fsA_resp = rpc.get_utxos_by_addresses(vec![fsA_addr]).await.expect("get_utxos fsA");
    if fsA_resp.is_empty() { println!("[2x2] no for-sale UTXO for listing A ({AMOUNT_A}). aborting."); return; }
    let fsA_entry = &fsA_resp[0];
    let fsA_outpoint = TransactionOutpoint::from(fsA_entry.outpoint.clone());
    let fsA_value = fsA_entry.utxo_entry.amount;
    println!("[2x2] for-sale A utxo {}:{}  value {}", fsA_outpoint.transaction_id, fsA_outpoint.index, fsA_value);

    let fsB_addr = extract_script_pub_key_address(&fsB_p2sh, Prefix::Testnet).expect("fsB addr");
    let fsB_resp = rpc.get_utxos_by_addresses(vec![fsB_addr]).await.expect("get_utxos fsB");
    if fsB_resp.is_empty() { println!("[2x2] no for-sale UTXO for listing B ({AMOUNT_B}). aborting."); return; }
    let fsB_entry = &fsB_resp[0];
    let fsB_outpoint = TransactionOutpoint::from(fsB_entry.outpoint.clone());
    let fsB_value = fsB_entry.utxo_entry.amount;
    println!("[2x2] for-sale B utxo {}:{}  value {}", fsB_outpoint.transaction_id, fsB_outpoint.index, fsB_value);

    // Find each buyer's plain funding UTXO.
    let buyer_a_spk = pay_to_address_script(&buyer_a_addr);
    let a_resp = rpc.get_utxos_by_addresses(vec![buyer_a_addr.clone()]).await.expect("get_utxos buyerA");
    if a_resp.is_empty() { println!("[2x2] buyer A has no plain funding UTXO. faucet {buyer_a_addr}. aborting."); return; }
    let a_entry = &a_resp[0];
    let a_outpoint = TransactionOutpoint::from(a_entry.outpoint.clone());
    let a_value = a_entry.utxo_entry.amount;
    if a_value <= PAYMENT + FEE { println!("[2x2] buyer A funds {a_value} <= payment+fee. aborting."); return; }
    println!("[2x2] buyer A funding utxo {}:{}  value {}", a_outpoint.transaction_id, a_outpoint.index, a_value);

    let buyer_b_spk = pay_to_address_script(&buyer_b_addr);
    let b_resp = rpc.get_utxos_by_addresses(vec![buyer_b_addr.clone()]).await.expect("get_utxos buyerB");
    if b_resp.is_empty() { println!("[2x2] buyer B has no plain funding UTXO. faucet {buyer_b_addr}. aborting."); return; }
    let b_entry = &b_resp[0];
    let b_outpoint = TransactionOutpoint::from(b_entry.outpoint.clone());
    let b_value = b_entry.utxo_entry.amount;
    if b_value <= PAYMENT + FEE { println!("[2x2] buyer B funds {b_value} <= payment+fee. aborting."); return; }
    println!("[2x2] buyer B funding utxo {}:{}  value {}", b_outpoint.transaction_id, b_outpoint.index, b_value);

    let cov_id = covenant_id();
    let fsA_utxo = UtxoEntry::new(fsA_value, fsA_p2sh.clone(), 0, false, Some(cov_id));
    let fsB_utxo = UtxoEntry::new(fsB_value, fsB_p2sh.clone(), 0, false, Some(cov_id));
    let a_utxo = UtxoEntry::new(a_value, buyer_a_spk.clone(), 0, false, None);
    let b_utxo = UtxoEntry::new(b_value, buyer_b_spk.clone(), 0, false, None);

    // Outputs (order is load-bearing for the witness mapping):
    //  0: token A -> buyer A   1: token B -> buyer B
    //  2: payment -> seller (buyer A pays)   3: payment -> seller (buyer B pays)
    //  4: change -> buyer A    5: change -> buyer B
    let seller_spk = pay_to_address_script(&seller_addr);
    let out0 = TransactionOutput { value: fsA_value, script_public_key: tokA_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }) };
    let out1 = TransactionOutput { value: fsB_value, script_public_key: tokB_p2sh, covenant: Some(CovenantBinding { authorizing_input: 1, covenant_id: cov_id }) };
    let out2 = TransactionOutput { value: PAYMENT, script_public_key: seller_spk.clone(), covenant: None };
    let out3 = TransactionOutput { value: PAYMENT, script_public_key: seller_spk.clone(), covenant: None };
    let a_change = a_value - PAYMENT - FEE;
    let b_change = b_value - PAYMENT - FEE;
    let out4 = TransactionOutput { value: a_change, script_public_key: buyer_a_spk.clone(), covenant: None };
    let out5 = TransactionOutput { value: b_change, script_public_key: buyer_b_spk.clone(), covenant: None };
    let outputs = vec![out0, out1, out2, out3, out4, out5];

    // Unsigned skeleton (all inputs empty sigscript) for the buyer sighashes.
    let in0u = TransactionInput { previous_outpoint: fsA_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let in1u = TransactionInput { previous_outpoint: fsB_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let in2u = TransactionInput { previous_outpoint: a_outpoint, signature_script: vec![], sequence: 0, mass: plain_mass };
    let in3u = TransactionInput { previous_outpoint: b_outpoint, signature_script: vec![], sequence: 0, mass: plain_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0u, in1u, in2u, in3u], outputs.clone(), 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let entries = vec![fsA_utxo.clone(), fsB_utxo.clone(), a_utxo.clone(), b_utxo.clone()];
    let mtx = MutableTransaction::with_entries(unsigned, entries.clone());
    let reused = SigHashReusedValuesUnsync::new();

    // Sign input 2 (buyer A funding) and input 3 (buyer B funding).
    let a_sh = calc_schnorr_signature_hash(&mtx.as_verifiable(), 2, SIG_HASH_ALL, &reused);
    let a_msg = secp256k1::Message::from_digest_slice(a_sh.as_bytes().as_slice()).expect("a sighash");
    let mut a_sig = buyer_a.sign_schnorr(a_msg).as_ref().to_vec();
    a_sig.push(SIG_HASH_ALL.to_u8());
    let in2_ss = ScriptBuilder::new().add_data(&a_sig).expect("push A sig").drain();

    let b_sh = calc_schnorr_signature_hash(&mtx.as_verifiable(), 3, SIG_HASH_ALL, &reused);
    let b_msg = secp256k1::Message::from_digest_slice(b_sh.as_bytes().as_slice()).expect("b sighash");
    let mut b_sig = buyer_b.sign_schnorr(b_msg).as_ref().to_vec();
    b_sig.push(SIG_HASH_ALL.to_u8());
    let in3_ss = ScriptBuilder::new().add_data(&b_sig).expect("push B sig").drain();

    // COVENANT ABI (multi-input): input 0 is the LEADER and carries the COMPLETE arrays for ALL
    // covenant inputs/outputs; input 1 is a DELEGATE with empty args (is_leader:false). This matches
    // the proven merge pattern in kcc20_tests.rs (leader = full args, delegate = vec![]).
    //   newStates = [token A -> buyer A, token B -> buyer B]   (one per covenant OUTPUT)
    //   sigs      = [dummy, dummy]                              (one per covenant INPUT; for-sale = dummy)
    //   witnesses = [2, 3]  (one BYTE per covenant input; flat byte[]; strictly increasing -> F1)
    let new_states: Expr = vec![
        new_state(buyer_a_pk.clone(), AMOUNT_A, false, 0),
        new_state(buyer_b_pk.clone(), AMOUNT_B, false, 0),
    ].into();
    let sigs: Expr = vec![Expr::bytes(vec![0u8; 65]), Expr::bytes(vec![0u8; 65])].into();
    let witnesses: Expr = Expr::bytes(vec![2u8, 3u8]); // input 0 -> output 2, input 1 -> output 3

    // Input 0: LEADER (full arrays).
    let mut in0_ss = fsA_compiled.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("build leader sigscript");
    let redeemA = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&fsA_compiled.script).expect("push redeemA").drain();
    in0_ss.extend_from_slice(&redeemA);

    // Input 1: DELEGATE (empty args, is_leader:false). Still reveals its own redeem script.
    let mut in1_ss = fsB_compiled.build_sig_script_for_covenant_decl("transfer", vec![], CovenantDeclCallOptions { is_leader: false }).expect("build delegate sigscript");
    let redeemB = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&fsB_compiled.script).expect("push redeemB").drain();
    in1_ss.extend_from_slice(&redeemB);

    let in0 = TransactionInput { previous_outpoint: fsA_outpoint, signature_script: in0_ss, sequence: 0, mass: cov_mass };
    let in1 = TransactionInput { previous_outpoint: fsB_outpoint, signature_script: in1_ss, sequence: 0, mass: cov_mass };
    let in2 = TransactionInput { previous_outpoint: a_outpoint, signature_script: in2_ss, sequence: 0, mass: plain_mass };
    let in3 = TransactionInput { previous_outpoint: b_outpoint, signature_script: in3_ss, sequence: 0, mass: plain_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0, in1, in2, in3], outputs, 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[2x2] claim tx id: {}", tx.id());
    println!("[2x2] witnesses = [2, 3] (strictly increasing); seller paid {PAYMENT} on EACH of outputs 2 and 3");
    println!("[2x2] buyer A change {a_change}; buyer B change {b_change}; fee {FEE}");

    // LOCAL VM VERIFY both covenant inputs (the payment-authorized, F1-gated spends).
    println!("[2x2] local VM verify (covenant input 0)...");
    match local_vm_verify(&tx, entries.clone(), 0) {
        Ok(()) => println!("[2x2] input 0: PASS"),
        Err(e) => { println!("[2x2] input 0: FAIL — {e:?}"); println!("[2x2] aborting."); return; }
    }
    println!("[2x2] local VM verify (covenant input 1)...");
    match local_vm_verify(&tx, entries.clone(), 1) {
        Ok(()) => println!("[2x2] input 1: PASS — F1 strict-increasing accepted."),
        Err(e) => { println!("[2x2] input 1: FAIL — {e:?}"); println!("[2x2] aborting."); return; }
    }

    if !go {
        println!();
        println!("[2x2] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[2x2] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[2x2] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[2x2] REJECTED: {e}"),
    }
}
