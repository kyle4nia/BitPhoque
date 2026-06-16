// BitPhoque — REJECTION TESTS (accept AND reject; the real proof the gate works).
//
// Self-contained, NO networking. Fabricates the spend locally (like kcc20_tests.rs)
// and runs each tx through the in-process TxScriptEngine with covenants enabled.
//
// Three cases — the gate is proven ONLY if all three behave:
//   CONTROL    (must PASS): valid handoff, conservation holds, signed by owner.
//   REJECT #1  (must FAIL): amount inflated (conservation break) — checkAmounts must reject.
//   REJECT #2  (must FAIL): signed by the WRONG key — checkSigs must reject.
//
// "PASS" from the control alone is half-proven. A covenant that accepts good input
// but also accepts bad input is hollow. This proves the enforcement is real, not a no-op.
//
// Run: cargo run -p silverscript-lang --bin bitphoque_reject_tests

use std::fs;

use kaspa_consensus_core::{
    hashing::sighash::{calc_schnorr_signature_hash, SigHashReusedValuesUnsync},
    hashing::sighash_type::SIG_HASH_ALL,
    tx::{
        CovenantBinding, MutableTransaction, PopulatedTransaction, Transaction, TransactionId,
        TransactionInput, TransactionOutpoint, TransactionOutput, UtxoEntry, VerifiableTransaction,
    },
    Hash,
};
use kaspa_txscript::caches::Cache;
use kaspa_txscript::covenants::CovenantsContext;
use kaspa_txscript::{pay_to_script_hash_script, script_builder::ScriptBuilder, EngineCtx, EngineFlags, TxScriptEngine};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::Keypair;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CompiledContract, CovenantDeclCallOptions};

const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque.sil";

// A fabricated covenant_id constant (matches on both input UTXO and output binding,
// exactly as kcc20_tests.rs uses COV_A — the local VM does not re-derive it).
const COV_A: Hash = Hash::from_bytes(*b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;

fn ctor_args(owner: Vec<u8>, amount: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner),
        Expr::int(amount),
        Expr::byte(0),
        Expr::bool(false),
        Expr::int(MAX_COV_INS),
        Expr::int(MAX_COV_OUTS),
    ]
}

fn new_state(owner: Vec<u8>, amount: i64) -> Expr<'static> {
    struct_object(vec![
        ("ownerIdentifier", Expr::bytes(owner)),
        ("identifierType", Expr::byte(0)),
        ("amount", Expr::int(amount)),
        ("isMinter", Expr::bool(false)),
    ])
}

// Sign input 0 over the given tx with the given key (schnorr, SIG_HASH_ALL, +type byte).
fn sign_input(tx: Transaction, entries: Vec<UtxoEntry>, key: &Keypair) -> Vec<u8> {
    let mtx = MutableTransaction::with_entries(tx, entries);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = key.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());
    signature
}

// Run input 0 through the in-process covenant VM. Ok = accepted, Err = rejected.
fn vm_verify(tx: &Transaction, entries: Vec<UtxoEntry>) -> Result<(), TxScriptError> {
    let reused = SigHashReusedValuesUnsync::new();
    let sig_cache = Cache::new(10_000);
    let input = tx.inputs[0].clone();
    let populated = PopulatedTransaction::new(tx, entries);
    let cov_ctx = CovenantsContext::from_tx(&populated).map_err(TxScriptError::from)?;
    let utxo = populated.utxo(0).expect("input utxo");
    let mut vm = TxScriptEngine::from_transaction_input(
        &populated, &input, 0, utxo,
        EngineCtx::new(&sig_cache).with_reused(&reused).with_covenants_ctx(&cov_ctx),
        EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() },
    );
    vm.execute()
}

// Build a handoff spend: genesis(owner=current, amount=in_amt) -> output(owner=recipient, amount=out_amt),
// signed by `signer`. Returns (tx, entries) ready for vm_verify. Varying in_amt!=out_amt breaks
// conservation; varying signer!=current owner breaks the signature check.
fn build_handoff(
    source: &str,
    current: &Keypair,
    in_amt: i64,
    recipient_pk: Vec<u8>,
    out_amt: i64,
    signer: &Keypair,
) -> (Transaction, Vec<UtxoEntry>) {
    let current_pk = current.x_only_public_key().0.serialize().to_vec();
    let genesis: CompiledContract = compile_contract(source, &ctor_args(current_pk, in_amt), CompileOptions::default()).expect("compile genesis");
    let genesis_p2sh = pay_to_script_hash_script(&genesis.script);

    // The output must pay to the RECIPIENT's compiled contract P2SH (owner=recipient, amount=out_amt),
    // matching how kcc20_tests.rs builds the handoff. Using the genesis P2SH here makes the
    // covenant's output-state validation reject even a valid spend (VerifyError).
    let recipient: CompiledContract = compile_contract(source, &ctor_args(recipient_pk.clone(), out_amt), CompileOptions::default()).expect("compile recipient");
    let recipient_p2sh = pay_to_script_hash_script(&recipient.script);

    let outpoint = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, genesis_p2sh, 0, false, Some(COV_A))];

    let output = TransactionOutput {
        value: 1_000,
        script_public_key: recipient_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }),
    };

    // Unsigned (empty sigscript), sign with `signer`.
    let unsigned = Transaction::new(1, vec![TransactionInput::new(outpoint, vec![], 0, 0)], vec![output.clone()], 0, Default::default(), 0, vec![]);
    let signature = sign_input(unsigned, entries.clone(), signer);

    // Leader sigscript: new-state array (recipient, out_amt), sigs, witnesses + pushed redeem.
    let new_states: Expr = vec![new_state(recipient_pk, out_amt)].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = genesis
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&genesis.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let tx = Transaction::new(1, vec![TransactionInput::new(outpoint, sigscript, 0, 0)], vec![output], 0, Default::default(), 0, vec![]);
    (tx, entries)
}

fn main() {
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let current = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let recipient = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let wrong = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let recipient_pk = recipient.x_only_public_key().0.serialize().to_vec();

    let mut all_ok = true;

    // CONTROL: 1000 -> 1000, signed by owner. MUST PASS.
    let (tx, entries) = build_handoff(&source, &current, 1_000, recipient_pk.clone(), 1_000, &current);
    match vm_verify(&tx, entries) {
        Ok(()) => println!("[CONTROL ] PASS (expected PASS) — valid handoff accepted. OK"),
        Err(e) => { println!("[CONTROL ] FAIL (expected PASS) — {e:?}. *** UNEXPECTED ***"); all_ok = false; }
    }

    // REJECT #1: 1000 -> 50000 (conservation break), signed by owner. MUST FAIL.
    let (tx, entries) = build_handoff(&source, &current, 1_000, recipient_pk.clone(), 50_000, &current);
    match vm_verify(&tx, entries) {
        Ok(()) => { println!("[REJECT 1] PASS (expected FAIL) — inflation ACCEPTED. *** CONSERVATION IS A NO-OP ***"); all_ok = false; }
        Err(e) => println!("[REJECT 1] FAIL (expected FAIL) — inflation rejected: {e:?}. OK"),
    }

    // REJECT #2: 1000 -> 1000 but signed by WRONG key. MUST FAIL.
    let (tx, entries) = build_handoff(&source, &current, 1_000, recipient_pk.clone(), 1_000, &wrong);
    match vm_verify(&tx, entries) {
        Ok(()) => { println!("[REJECT 2] PASS (expected FAIL) — wrong sig ACCEPTED. *** SIGNATURE CHECK IS A NO-OP ***"); all_ok = false; }
        Err(e) => println!("[REJECT 2] FAIL (expected FAIL) — wrong sig rejected: {e:?}. OK"),
    }

    println!();
    if all_ok {
        println!("[RESULT] GATE PROVEN: accepts valid, rejects inflation AND wrong-sig. Enforcement is real.");
    } else {
        println!("[RESULT] *** GATE NOT PROVEN — see UNEXPECTED/NO-OP lines above. DO NOT trust this contract. ***");
    }
}
