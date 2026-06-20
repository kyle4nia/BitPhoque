// BitPhoque — MERGE 2->1 harness v3 (TN10 practice). Consolidate two parcels into one.
//
// The inverse of the split: spends TWO not-for-sale, SAME-owner v3 covenant UTXOs and creates
// ONE new not-for-sale covenant UTXO (same owner) carrying the summed phoques. Owner-SIGNED on
// BOTH inputs (each input is not-for-sale -> checkAuth's checkSig branch -> needs the owner sig).
//
// WHY THIS IS A SMALL DIFF, NOT A NEW BUILD:
//  - Multi-covenant-input ABI (leader/delegate) is the SAME machinery proven by the 2x2 claim
//    (input 0 = leader carries the COMPLETE arrays; input 1 = delegate, empty args; each pushes
//    its own redeem script). The 2x2 claim used UNSIGNED for-sale inputs (dummy sigs, witnesses
//    drove auth). Merge uses SIGNED not-for-sale inputs: real sigs in the leader's sigs array,
//    and the witnesses array is DORMANT FILLER (checkAuth never indexes it when no input is
//    for-sale — verified by reading bitphoque_sale_v3.sil's checkAuth).
//  - Single owner-signed covenant spend is the SAME machinery proven by split/list.
//  So: "2x2 claim's input structure" + "split's owner-signed authorization" + "one output".
//
// CONTRACT CHECK (read from source before building):
//  - checkAuth: both inputs !forSale -> checkSig(sigs[i], owner) each. for-sale/witness branch
//    never entered; F1 accumulator dormant.
//  - checkAmounts: sum(inputs) == sum(outputs). 20033 + 5268 == 25301. No rule that #outputs
//    must equal/exceed #inputs, so 2->1 is fine.
//  - checkMintingTransfer: output !isMinter. fine.
//  - checkSaleState: anyInputForSale=false -> the forbid-for-sale-outputs block is skipped. fine.
//  - transfer binding from=maxCovIns(3) to=maxCovOuts(3): 2 in <= 3, 1 out <= 3. within caps.
//
// CLI:  merge <amount0> <amount1> <owner_keyfile> [--go]
//   amount0, amount1 = the phoque counts of the two not-for-sale parcels to merge (each a
//   DIFFERENT P2SH because amount bakes into the script). Output amount = amount0 + amount1.
//   owner_keyfile = the OWNER of BOTH parcels (merge is owner-signed; only the owner can merge
//   what they own). Both parcels MUST share the same owner (this owner).
//
//   e.g.  ... -- 20033 5268 C:\bitphoque\bitphoque_buyer1.key          (dry run)
//         ... -- 20033 5268 C:\bitphoque\bitphoque_buyer1.key --go     (broadcast)
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
use kaspa_txscript::{pay_to_script_hash_script, extract_script_pub_key_address, script_builder::ScriptBuilder, EngineCtx, EngineFlags, TxScriptEngine};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::Keypair;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CovenantDeclCallOptions};

const NODE: &str = "192.168.0.206:16210";
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COVENANT_ID_HEX: &str = "3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458";

const FEE: u64 = 1_300_000;     // 2 covenant inputs (TWO 2793B script reveals), 1 output. Node-confirmed
                                // floor 1,244,500 (compute mass 12445) — HEAVIER than the single
                                // 2-input claim (857k) because the 2nd input is a full script reveal,
                                // not a cheap plain input. Use 1.3M (margin over the confirmed floor).
const MAX_COV_INS: i64 = 3;     // MUST match genesis/split/listing (baked into P2SH)
const MAX_COV_OUTS: i64 = 3;
const COVENANT_SCRIPT_UNITS: u64 = 300_000;

// 8-arg ctor: owner, amount, idType=0, isMinter=false, forSale, price, maxIns, maxOuts.
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
    // CLI: <amount0> <amount1> <owner_keyfile> [--go]
    let raw: Vec<String> = std::env::args().collect();
    let go = raw.iter().any(|a| a == "--go");
    let pos: Vec<&String> = raw.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.len() < 3 {
        eprintln!("usage: bitphoque_sale_merge_v3 <amount0> <amount1> <owner_keyfile> [--go]");
        eprintln!("  e.g. 20033 5268 C:\\bitphoque\\bitphoque_buyer1.key");
        std::process::exit(2);
    }
    let amount0: i64 = pos[0].parse().expect("amount0 must be an integer (e.g. 20033)");
    let amount1: i64 = pos[1].parse().expect("amount1 must be an integer (e.g. 5268)");
    let key_file = pos[2].as_str();
    let out_amount = amount0 + amount1;
    println!("[merge] merging parcels: {amount0} + {amount1} = {out_amount} phoques  owner key: {key_file}");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let cov_budget = ComputeBudget::checked_covering_script_units(ScriptUnits(COVENANT_SCRIPT_UNITS)).expect("budget fits");
    let cov_mass = TxInputMass::ComputeBudget(cov_budget);

    let owner = load_key(key_file);
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    println!("[merge] owner pubkey: {}", faster_hex::hex_string(&owner_pk));
    let owner_addr = Address::new(Prefix::Testnet, Version::PubKey, &owner_pk);
    println!("[merge] owner P2PK addr: {owner_addr}");

    // Two input contracts: same owner, NOT for sale, at each amount (each a different P2SH).
    let in0_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount0, false, 0), CompileOptions::default()).expect("compile in0");
    let in0_p2sh = pay_to_script_hash_script(&in0_compiled.script);
    let in1_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), amount1, false, 0), CompileOptions::default()).expect("compile in1");
    let in1_p2sh = pay_to_script_hash_script(&in1_compiled.script);

    // Output contract: same owner, NOT for sale, summed amount.
    let out_compiled = compile_contract(&source, &ctor_args(owner_pk.clone(), out_amount, false, 0), CompileOptions::default()).expect("compile out");
    let out_p2sh = pay_to_script_hash_script(&out_compiled.script);

    let subscription_context = SubscriptionContext::new();
    println!("[merge] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct, format!("grpc://{NODE}"),
        Some(subscription_context.clone()), true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[merge] connected.");

    // Find the two not-for-sale covenant UTXOs.
    let in0_addr = extract_script_pub_key_address(&in0_p2sh, Prefix::Testnet).expect("in0 addr");
    println!("[merge] looking for parcel 0 ({amount0} phoques, not-for-sale) at {in0_addr}");
    let in0_resp = rpc.get_utxos_by_addresses(vec![in0_addr]).await.expect("get_utxos in0");
    if in0_resp.is_empty() { println!("[merge] no not-for-sale UTXO for parcel 0 ({amount0}). aborting."); return; }
    let in0_entry = &in0_resp[0];
    let in0_outpoint = TransactionOutpoint::from(in0_entry.outpoint.clone());
    let in0_value = in0_entry.utxo_entry.amount;
    println!("[merge] parcel 0 utxo {}:{}  value {}", in0_outpoint.transaction_id, in0_outpoint.index, in0_value);

    let in1_addr = extract_script_pub_key_address(&in1_p2sh, Prefix::Testnet).expect("in1 addr");
    println!("[merge] looking for parcel 1 ({amount1} phoques, not-for-sale) at {in1_addr}");
    let in1_resp = rpc.get_utxos_by_addresses(vec![in1_addr]).await.expect("get_utxos in1");
    if in1_resp.is_empty() { println!("[merge] no not-for-sale UTXO for parcel 1 ({amount1}). aborting."); return; }
    let in1_entry = &in1_resp[0];
    let in1_outpoint = TransactionOutpoint::from(in1_entry.outpoint.clone());
    let in1_value = in1_entry.utxo_entry.amount;
    println!("[merge] parcel 1 utxo {}:{}  value {}", in1_outpoint.transaction_id, in1_outpoint.index, in1_value);

    let cov_id = covenant_id();
    let in0_utxo = UtxoEntry::new(in0_value, in0_p2sh.clone(), 0, false, Some(cov_id));
    let in1_utxo = UtxoEntry::new(in1_value, in1_p2sh.clone(), 0, false, Some(cov_id));

    // Single covenant output: summed phoques, summed value minus fee, authorizing_input 0.
    let out_value = in0_value + in1_value - FEE;
    let output = TransactionOutput {
        value: out_value,
        script_public_key: out_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };
    println!("[merge] output: {out_amount} phoques, value {out_value} (= {in0_value} + {in1_value} - fee {FEE})");

    // Unsigned skeleton (both inputs empty sigscript) for the two covenant-input sighashes.
    let in0u = TransactionInput { previous_outpoint: in0_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let in1u = TransactionInput { previous_outpoint: in1_outpoint, signature_script: vec![], sequence: 0, mass: cov_mass };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0u, in1u], vec![output.clone()], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    let entries = vec![in0_utxo.clone(), in1_utxo.clone()];
    let mtx = MutableTransaction::with_entries(unsigned, entries.clone());
    let reused = SigHashReusedValuesUnsync::new();

    // Sign BOTH covenant inputs with the owner key (both not-for-sale -> checkSig branch).
    // CRITICAL (from the proven kcc20_tests.rs merge pattern): in the leader/delegate ABI the
    // covenant verifies ALL input signatures from the LEADER's position, so BOTH sigs are computed
    // against input INDEX 0 — NOT 0 and 1. (Signing sig1 against input 1 yields a different sighash
    // and the covenant rejects with VerifyError. This is the first tx that signs two covenant
    // inputs, so it's the first time this surfaces — the 2x2 claim's covenant inputs were unsigned.)
    let sh0 = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg0 = secp256k1::Message::from_digest_slice(sh0.as_bytes().as_slice()).expect("sighash0");
    let mut sig0 = owner.sign_schnorr(msg0).as_ref().to_vec();
    sig0.push(SIG_HASH_ALL.to_u8());

    // sig1 ALSO signs input 0 (leader position), per the proven merge pattern.
    let mut sig1 = owner.sign_schnorr(msg0).as_ref().to_vec();
    sig1.push(SIG_HASH_ALL.to_u8());

    // COVENANT ABI (multi-input): input 0 = LEADER with the COMPLETE arrays; input 1 = DELEGATE.
    //   newStates = [merged token -> owner]   (one per covenant OUTPUT = 1)
    //   sigs      = [sig0, sig1]               (one per covenant INPUT = 2; REAL owner sigs)
    //   witnesses = [0, 1]                     (one byte per covenant input; DORMANT — no for-sale
    //                                           input, so checkAuth never indexes it. Matches the
    //                                           PROVEN kcc20_tests.rs merge pattern, which uses
    //                                           [0,1] — match proven over reasoning about what
    //                                           "shouldn't" matter.)
    let new_states: Expr = vec![new_state(owner_pk.clone(), out_amount, false, 0)].into();
    let sigs: Expr = vec![Expr::bytes(sig0), Expr::bytes(sig1)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8, 1u8]);

    // Input 0: LEADER (full arrays).
    let mut in0_ss = in0_compiled.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("build leader sigscript");
    let redeem0 = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in0_compiled.script).expect("push redeem0").drain();
    in0_ss.extend_from_slice(&redeem0);

    // Input 1: DELEGATE (empty args, is_leader:false). Still reveals its own redeem script.
    let mut in1_ss = in1_compiled.build_sig_script_for_covenant_decl("transfer", vec![], CovenantDeclCallOptions { is_leader: false }).expect("build delegate sigscript");
    let redeem1 = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in1_compiled.script).expect("push redeem1").drain();
    in1_ss.extend_from_slice(&redeem1);

    let in0 = TransactionInput { previous_outpoint: in0_outpoint, signature_script: in0_ss, sequence: 0, mass: cov_mass };
    let in1 = TransactionInput { previous_outpoint: in1_outpoint, signature_script: in1_ss, sequence: 0, mass: cov_mass };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in0, in1], vec![output], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[merge] merge tx id: {}", tx.id());
    println!("[merge] CONSERVATION: {amount0} + {amount1} = {out_amount} phoques (contract enforces totalIn==totalOut)");

    // LOCAL VM VERIFY both covenant inputs (both owner-signed not-for-sale spends).
    println!("[merge] local VM verify (covenant input 0)...");
    match local_vm_verify(&tx, entries.clone(), 0) {
        Ok(()) => println!("[merge] input 0: PASS"),
        Err(e) => { println!("[merge] input 0: FAIL — {e:?}"); println!("[merge] aborting."); return; }
    }
    println!("[merge] local VM verify (covenant input 1)...");
    match local_vm_verify(&tx, entries.clone(), 1) {
        Ok(()) => println!("[merge] input 1: PASS — both signed inputs accepted, 2->1 conserved."),
        Err(e) => { println!("[merge] input 1: FAIL — {e:?}"); println!("[merge] aborting."); return; }
    }

    if !go {
        println!();
        println!("[merge] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[merge] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[merge] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[merge] REJECTED: {e}"),
    }
}
