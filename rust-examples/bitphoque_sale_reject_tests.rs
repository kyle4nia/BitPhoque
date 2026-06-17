// BitPhoque — SALE + LISTING REJECTION TESTS (v2 contract).
//
// Proves BOTH halves of the for-sale mechanism at the VM level:
//   CLAIM path (payment-authorized, UNSIGNED): a for-sale UTXO is spent by paying the seller.
//   LISTING path (owner-SIGNED): a not-for-sale UTXO is flipped to for-sale by its owner.
//
// Compiling proved nothing. This proves enforcement. The gate is proven ONLY if every case behaves.
//
// CLAIM tests (regression from v1 — must still hold under v2):
//   C-CONTROL (PASS): listed UTXO claimed, pay >= price to seller, buyer owns result, forSale reset.
//   C1 underpay / C2 wrong-payee / C3 no-payment / C4 not-listed / C5 stays-for-sale / C6 output-match (all FAIL).
// LISTING tests (new in v2):
//   L-CONTROL (PASS): owner signs, flips own not-for-sale UTXO to for-sale. Listing WORKS (v1 couldn't).
//   L1 no-sig (FAIL): create a for-sale output from a not-for-sale input with NO owner signature.
//   L2 wrong-sig (FAIL): same but signed by the wrong key.
//
// Run: cargo run -p silverscript-lang --bin bitphoque_sale_reject_tests

use std::fs;

use kaspa_consensus_core::{
    tx::{
        CovenantBinding, MutableTransaction, PopulatedTransaction, ScriptPublicKey, Transaction,
        TransactionId, TransactionInput, TransactionOutpoint, TransactionOutput, UtxoEntry,
        VerifiableTransaction,
    },
    hashing::sighash::{calc_schnorr_signature_hash, SigHashReusedValuesUnsync},
    hashing::sighash_type::SIG_HASH_ALL,
    Hash,
};
use kaspa_txscript::caches::Cache;
use kaspa_txscript::covenants::CovenantsContext;
use kaspa_txscript::{pay_to_script_hash_script, script_builder::ScriptBuilder, EngineCtx, EngineFlags, TxScriptEngine};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::Keypair;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CompiledContract, CovenantDeclCallOptions};

const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v2.sil";
const COV_A: Hash = Hash::from_bytes(*b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;

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

// Sign input 0 (schnorr, SIG_HASH_ALL, +type byte). Used by the LISTING path.
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

fn p2pk_spk(pk: &[u8]) -> ScriptPublicKey {
    let script = ScriptBuilder::new().add_data(pk).expect("push pk").add_op(0xac).expect("op").drain();
    ScriptPublicKey::new(0, script.into())
}

// ---------------- CLAIM path (payment-authorized, unsigned) ----------------

struct SaleParams {
    in_for_sale: bool,
    price: i64,
    pay_value: u64,
    pay_to_seller: bool,
    include_payment: bool,
    out_for_sale: bool,
    witness_index: u8,
}

fn build_sale(source: &str, seller: &Keypair, buyer_pk: Vec<u8>, amount: i64, p: &SaleParams) -> (Transaction, Vec<UtxoEntry>) {
    let seller_pk = seller.x_only_public_key().0.serialize().to_vec();
    let listed: CompiledContract = compile_contract(source, &ctor_args(seller_pk.clone(), amount, p.in_for_sale, p.price), CompileOptions::default()).expect("compile listed");
    let listed_p2sh = pay_to_script_hash_script(&listed.script);
    let buyer_contract: CompiledContract = compile_contract(source, &ctor_args(buyer_pk.clone(), amount, p.out_for_sale, 0), CompileOptions::default()).expect("compile buyer");
    let buyer_p2sh = pay_to_script_hash_script(&buyer_contract.script);

    let outpoint = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, listed_p2sh, 0, false, Some(COV_A))];
    let token_out = TransactionOutput { value: 1_000, script_public_key: buyer_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) };

    let mut outputs = vec![token_out];
    if p.include_payment {
        let payee = if p.pay_to_seller { &seller_pk } else { &buyer_pk };
        outputs.push(TransactionOutput { value: p.pay_value, script_public_key: p2pk_spk(payee), covenant: None });
    }

    let new_states: Expr = vec![new_state(buyer_pk, amount, p.out_for_sale, 0)].into();
    let sigs: Expr = vec![Expr::bytes(vec![0u8; 65])].into();
    let witnesses: Expr = Expr::bytes(vec![p.witness_index]);
    let mut sigscript = listed.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("build sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&listed.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);
    let tx = Transaction::new(1, vec![TransactionInput::new(outpoint, sigscript, 0, 0)], outputs, 0, Default::default(), 0, vec![]);
    (tx, entries)
}

// ---------------- LISTING path (owner-signed) ----------------

struct ListParams {
    out_for_sale: bool,   // output flagged for-sale (true = a listing)
    price: i64,           // listed price on the output
    signer: SignWith,     // who signs the listing
}
enum SignWith { Owner, Wrong, None }

// Build a listing: spend a NOT-for-sale UTXO owned by `owner`, produce an output (still owned by
// owner) flagged per params. Owner signs (or wrong/none per params).
fn build_listing(source: &str, owner: &Keypair, wrong: &Keypair, amount: i64, p: &ListParams) -> (Transaction, Vec<UtxoEntry>) {
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    // Input: NOT for sale, owned by owner.
    let input_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), amount, false, 0), CompileOptions::default()).expect("compile input");
    let input_p2sh = pay_to_script_hash_script(&input_c.script);
    // Output: still owned by owner, flagged per params (out_for_sale + price).
    let out_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), amount, p.out_for_sale, p.price), CompileOptions::default()).expect("compile out");
    let out_p2sh = pay_to_script_hash_script(&out_c.script);

    let outpoint = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, input_p2sh, 0, false, Some(COV_A))];
    let output = TransactionOutput { value: 1_000, script_public_key: out_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) };

    // Sign over the unsigned tx (input has empty sigscript) per the signer choice.
    let unsigned = Transaction::new(1, vec![TransactionInput::new(outpoint, vec![], 0, 0)], vec![output.clone()], 0, Default::default(), 0, vec![]);
    let signature = match p.signer {
        SignWith::Owner => sign_input(unsigned, entries.clone(), owner),
        SignWith::Wrong => sign_input(unsigned, entries.clone(), wrong),
        SignWith::None => vec![0u8; 65], // dummy, should fail the sig check
    };

    let new_states: Expr = vec![new_state(owner_pk, amount, p.out_for_sale, p.price)].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]); // not used on the signed (not-for-sale) branch
    let mut sigscript = input_c.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("build sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&input_c.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);
    let tx = Transaction::new(1, vec![TransactionInput::new(outpoint, sigscript, 0, 0)], vec![output], 0, Default::default(), 0, vec![]);
    (tx, entries)
}

fn main() {
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let seller = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let buyer = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let wrong = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let buyer_pk = buyer.x_only_public_key().0.serialize().to_vec();
    let amount = 1_000i64;
    let price = 500_000i64;
    let mut all_ok = true;

    let run = |label: &str, expect_pass: bool, res: Result<(), TxScriptError>, all_ok: &mut bool| {
        match (expect_pass, res) {
            (true, Ok(())) => println!("[{label}] PASS (expected PASS). OK"),
            (true, Err(e)) => { println!("[{label}] FAIL (expected PASS) — {e:?}. *** UNEXPECTED ***"); *all_ok = false; }
            (false, Err(_)) => println!("[{label}] FAIL (expected FAIL) — rejected. OK"),
            (false, Ok(())) => { println!("[{label}] PASS (expected FAIL) — *** CHEAT ACCEPTED — HOLE ***"); *all_ok = false; }
        }
    };

    println!("== CLAIM path (payment-authorized, unsigned) ==");
    let mk = |p: SaleParams| build_sale(&source, &seller, buyer_pk.clone(), amount, &p);
    let (tx, e) = mk(SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 1 });
    run("C-CONTROL", true, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = mk(SaleParams { in_for_sale: true, price, pay_value: (price as u64)-1, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 1 });
    run("C1 under  ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = mk(SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: false, include_payment: true, out_for_sale: false, witness_index: 1 });
    run("C2 payee  ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = mk(SaleParams { in_for_sale: true, price, pay_value: 0, pay_to_seller: true, include_payment: false, out_for_sale: false, witness_index: 0 });
    run("C3 nopay  ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = mk(SaleParams { in_for_sale: false, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 1 });
    run("C4 unlist ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = mk(SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: true, witness_index: 1 });
    run("C5 stays  ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = mk(SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 0 });
    run("C6 match  ", false, vm_verify(&tx, e), &mut all_ok);

    println!("== LISTING path (owner-signed) ==");
    let ml = |p: ListParams| build_listing(&source, &seller, &wrong, amount, &p);
    let (tx, e) = ml(ListParams { out_for_sale: true, price, signer: SignWith::Owner });
    run("L-CONTROL", true, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = ml(ListParams { out_for_sale: true, price, signer: SignWith::None });
    run("L1 nosig  ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = ml(ListParams { out_for_sale: true, price, signer: SignWith::Wrong });
    run("L2 wrong  ", false, vm_verify(&tx, e), &mut all_ok);

    println!();
    if all_ok {
        println!("[RESULT] SALE+LISTING GATE PROVEN: claim path rejects all cheats; listing requires owner sig; listing works.");
    } else {
        println!("[RESULT] *** GATE NOT PROVEN — see UNEXPECTED/HOLE lines. DO NOT deploy. ***");
    }
}
