// BitPhoque — SALE REJECTION TESTS (the safety case for the UNSIGNED payment-authorized path).
//
// The for-sale path lets a covenant UTXO move with NO owner signature, gated only by a payment
// check. That makes these rejection tests existential: if any cheat is ACCEPTED, anyone can drain
// for-sale phoques. Compiling proved nothing; THIS proves enforcement is real.
//
// Tests (gate proven ONLY if all behave):
//   CONTROL (PASS): for-sale UTXO claimed, pay >= price to seller, buyer owns result, forSale reset.
//   R1 underpay (FAIL): pay LESS than price.
//   R2 wrong payee (FAIL): pay >= price but to the WRONG address (not seller).
//   R3 no payment (FAIL): claim with no payment output.
//   R4 not-listed (FAIL): payment-claim a UTXO whose forSale=false (must require a sig instead).
//   R5 stays-for-sale (FAIL): claimed output keeps forSale=true (double-claim attack).
//   R6 output-match abuse (FAIL): witness points at a non-payment output (e.g. buyer's own) >= price.
//
// Run: cargo run -p silverscript-lang --bin bitphoque_sale_reject_tests

use std::fs;

use kaspa_consensus_core::{
    tx::{
        CovenantBinding, PopulatedTransaction, ScriptPublicKey, Transaction, TransactionId,
        TransactionInput, TransactionOutpoint, TransactionOutput, UtxoEntry, VerifiableTransaction,
    },
    hashing::sighash::SigHashReusedValuesUnsync,
    Hash,
};
use kaspa_txscript::caches::Cache;
use kaspa_txscript::covenants::CovenantsContext;
use kaspa_txscript::{pay_to_script_hash_script, script_builder::ScriptBuilder, EngineCtx, EngineFlags, TxScriptEngine};
use kaspa_txscript_errors::TxScriptError;
use secp256k1::Keypair;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CompiledContract, CovenantDeclCallOptions};

const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v1.sil";
const COV_A: Hash = Hash::from_bytes(*b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;

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

// State struct now carries forSale + price.
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

// A plain P2PK scriptPubKey for a payout to `pk` (x-only 32 bytes). Mirrors ScriptPubKeyP2PK:
// version 0, push of the 32-byte key + OpCheckSig. Built via ScriptBuilder to match the contract's
// `new ScriptPubKeyP2PK(...)`. NOTE: exact encoding must match what the .sil produces — verify.
fn p2pk_spk(pk: &[u8]) -> ScriptPublicKey {
    let script = ScriptBuilder::new().add_data(pk).expect("push pk").add_op(0xac /*OpCheckSig*/).expect("op").drain();
    ScriptPublicKey::new(0, script.into())
}

struct SaleParams {
    in_for_sale: bool,      // is the spent UTXO actually listed?
    price: i64,             // listed price (baked into the input contract)
    pay_value: u64,         // value of the payment output
    pay_to_seller: bool,    // does the payment go to the seller (true) or a wrong addr (false)?
    include_payment: bool,  // is there a payment output at all?
    out_for_sale: bool,     // does the resulting buyer UTXO keep forSale=true (cheat)?
    witness_index: u8,      // which output index the witness points at as "the payment"
}

// Build a sale-claim spend. Output 0 = phoques->buyer (covenant), Output 1 (optional) = payment.
fn build_sale(
    source: &str,
    seller: &Keypair,
    buyer_pk: Vec<u8>,
    amount: i64,
    p: &SaleParams,
) -> (Transaction, Vec<UtxoEntry>) {
    let seller_pk = seller.x_only_public_key().0.serialize().to_vec();
    // Input contract: owned by seller, listed per params, at `price`.
    let listed: CompiledContract = compile_contract(source, &ctor_args(seller_pk.clone(), amount, p.in_for_sale, p.price), CompileOptions::default()).expect("compile listed");
    let listed_p2sh = pay_to_script_hash_script(&listed.script);

    // Output 0: phoques -> buyer, forSale per params (cheat tests set true), price 0.
    let buyer_contract: CompiledContract = compile_contract(source, &ctor_args(buyer_pk.clone(), amount, p.out_for_sale, 0), CompileOptions::default()).expect("compile buyer");
    let buyer_p2sh = pay_to_script_hash_script(&buyer_contract.script);

    let outpoint = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, listed_p2sh, 0, false, Some(COV_A))];

    let token_out = TransactionOutput {
        value: 1_000,
        script_public_key: buyer_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }),
    };

    let mut outputs = vec![token_out];
    if p.include_payment {
        let payee = if p.pay_to_seller { &seller_pk } else { &buyer_pk };
        outputs.push(TransactionOutput {
            value: p.pay_value,
            script_public_key: p2pk_spk(payee),
            covenant: None,
        });
    }

    // For-sale path is payment-authorized: sig slot is a dummy (contract's else-branch never checks it).
    let new_states: Expr = vec![new_state(buyer_pk, amount, p.out_for_sale, 0)].into();
    let sigs: Expr = vec![Expr::bytes(vec![0u8; 65])].into();
    let witnesses: Expr = Expr::bytes(vec![p.witness_index]);
    let mut sigscript = listed
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&listed.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    let tx = Transaction::new(1, vec![TransactionInput::new(outpoint, sigscript, 0, 0)], outputs, 0, Default::default(), 0, vec![]);
    (tx, entries)
}

fn main() {
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let seller = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let buyer = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let buyer_pk = buyer.x_only_public_key().0.serialize().to_vec();
    let amount = 1_000i64;
    let price = 500_000i64;
    let mut all_ok = true;

    let mut check = |label: &str, expect_pass: bool, p: SaleParams, all_ok: &mut bool| {
        let (tx, entries) = build_sale(&source, &seller, buyer_pk.clone(), amount, &p);
        let res = vm_verify(&tx, entries);
        match (expect_pass, res) {
            (true, Ok(())) => println!("[{label}] PASS (expected PASS). OK"),
            (true, Err(e)) => { println!("[{label}] FAIL (expected PASS) — {e:?}. *** UNEXPECTED ***"); *all_ok = false; }
            (false, Err(_)) => println!("[{label}] FAIL (expected FAIL) — rejected. OK"),
            (false, Ok(())) => { println!("[{label}] PASS (expected FAIL) — *** CHEAT ACCEPTED — HOLE ***"); *all_ok = false; }
        }
    };

    // CONTROL: listed, pay full price to seller, buyer owns result, forSale reset, witness->output 1.
    check("CONTROL ", true, SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 1 }, &mut all_ok);
    // R1 underpay.
    check("R1 under ", false, SaleParams { in_for_sale: true, price, pay_value: (price as u64) - 1, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 1 }, &mut all_ok);
    // R2 wrong payee.
    check("R2 payee ", false, SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: false, include_payment: true, out_for_sale: false, witness_index: 1 }, &mut all_ok);
    // R3 no payment output (witness points at output 0, the token output).
    check("R3 nopay ", false, SaleParams { in_for_sale: true, price, pay_value: 0, pay_to_seller: true, include_payment: false, out_for_sale: false, witness_index: 0 }, &mut all_ok);
    // R4 not-listed UTXO claimed via payment path (no sig).
    check("R4 unlist", false, SaleParams { in_for_sale: false, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 1 }, &mut all_ok);
    // R5 result stays for-sale (double-claim attack).
    check("R5 stays ", false, SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: true, witness_index: 1 }, &mut all_ok);
    // R6 output-match abuse: witness points at output 0 (the buyer's token output) instead of a real payment.
    check("R6 match ", false, SaleParams { in_for_sale: true, price, pay_value: price as u64, pay_to_seller: true, include_payment: true, out_for_sale: false, witness_index: 0 }, &mut all_ok);

    println!();
    if all_ok {
        println!("[RESULT] SALE GATE PROVEN: accepts valid claim, rejects every cheat. Unsigned path is safe.");
    } else {
        println!("[RESULT] *** SALE GATE NOT PROVEN — see UNEXPECTED/HOLE lines. DO NOT deploy. ***");
    }
}
