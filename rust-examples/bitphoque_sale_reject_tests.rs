// BitPhoque — SALE + LISTING + FANOUT REJECTION TESTS (v3 contract).
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
// 2x2 FANOUT tests (NEW — generalize the sale beyond single-input/output):
//   F-CONTROL-claim (PASS): two for-sale inputs (distinct sellers), two distinct payment outputs, each >= price.
//   F-CONTROL-split (PASS): one not-for-sale input, owner-signed, two outputs (one for-sale = a listing, one kept).
//   F1 shared-output double-claim (FAIL): two for-sale inputs, witnesses=[2,2], ONE payment. Pay once, claim twice?
//   F2 cross-wired witness (FAIL): distinct sellers, B's input validated against A's payment output.
//   F3 fanout inflation (FAIL): 2-in 2-out, totalOut > totalIn.
//   F4 claim-leaves-listing (FAIL): 2-in claim, one output still forSale=true.
//   F5 mixed-state auth bypass (FAIL): input0 for-sale (paid), input1 NOT-for-sale but unsigned.
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

const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const COV_A: Hash = Hash::from_bytes(*b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;

fn ctor_args(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    ctor_args_b(owner, amount, for_sale, price, MAX_COV_INS, MAX_COV_OUTS)
}

// Bounded variant: explicit maxCovIns/maxCovOuts so the same builder serves 2x2 and 3x3.
fn ctor_args_b(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64, max_ins: i64, max_outs: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner),
        Expr::int(amount),
        Expr::byte(0),
        Expr::bool(false),
        Expr::bool(for_sale),
        Expr::int(price),
        Expr::int(max_ins),
        Expr::int(max_outs),
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

// Sign a given input (schnorr, SIG_HASH_ALL, +type byte). Used by the LISTING + fanout paths.
fn sign_input_idx(tx: Transaction, entries: Vec<UtxoEntry>, input_idx: usize, key: &Keypair) -> Vec<u8> {
    let mtx = MutableTransaction::with_entries(tx, entries);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), input_idx, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = key.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());
    signature
}

// Convenience: sign input 0 (the original 1x1 helper, preserved).
fn sign_input(tx: Transaction, entries: Vec<UtxoEntry>, key: &Keypair) -> Vec<u8> {
    sign_input_idx(tx, entries, 0, key)
}

// Verify a SINGLE input (1x1 path).
fn vm_verify(tx: &Transaction, entries: Vec<UtxoEntry>) -> Result<(), TxScriptError> {
    vm_verify_input(tx, entries, 0)
}

// Verify a SPECIFIC input index against the full populated tx.
fn vm_verify_input(tx: &Transaction, entries: Vec<UtxoEntry>, input_idx: usize) -> Result<(), TxScriptError> {
    let reused = SigHashReusedValuesUnsync::new();
    let sig_cache = Cache::new(10_000);
    let input = tx.inputs[input_idx].clone();
    let populated = PopulatedTransaction::new(tx, entries);
    let cov_ctx = CovenantsContext::from_tx(&populated).map_err(TxScriptError::from)?;
    let utxo = populated.utxo(input_idx).expect("input utxo");
    let mut vm = TxScriptEngine::from_transaction_input(
        &populated, &input, input_idx, utxo,
        EngineCtx::new(&sig_cache).with_reused(&reused).with_covenants_ctx(&cov_ctx),
        EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() },
    );
    vm.execute()
}

// Verify EVERY input; a 2x2 tx is valid only if ALL inputs pass. Returns the first error,
// so "expected FAIL" cases are satisfied if ANY input rejects.
fn vm_verify_all(tx: &Transaction, entries: Vec<UtxoEntry>, n_inputs: usize) -> Result<(), TxScriptError> {
    for idx in 0..n_inputs {
        vm_verify_input(tx, entries.clone(), idx)?;
    }
    Ok(())
}

fn p2pk_spk(pk: &[u8]) -> ScriptPublicKey {
    let script = ScriptBuilder::new().add_data(pk).expect("push pk").add_op(0xac).expect("op").drain();
    ScriptPublicKey::new(0, script.into())
}

// ---------------- CLAIM path (payment-authorized, unsigned) — 1x1 ----------------

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

// ---------------- LISTING path (owner-signed) — 1x1 ----------------

struct ListParams {
    out_for_sale: bool,   // output flagged for-sale (true = a listing)
    price: i64,           // listed price on the output
    signer: SignWith,     // who signs the listing
}
enum SignWith { Owner, Wrong, None }

fn build_listing(source: &str, owner: &Keypair, wrong: &Keypair, amount: i64, p: &ListParams) -> (Transaction, Vec<UtxoEntry>) {
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    let input_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), amount, false, 0), CompileOptions::default()).expect("compile input");
    let input_p2sh = pay_to_script_hash_script(&input_c.script);
    let out_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), amount, p.out_for_sale, p.price), CompileOptions::default()).expect("compile out");
    let out_p2sh = pay_to_script_hash_script(&out_c.script);

    let outpoint = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, input_p2sh, 0, false, Some(COV_A))];
    let output = TransactionOutput { value: 1_000, script_public_key: out_p2sh, covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) };

    let unsigned = Transaction::new(1, vec![TransactionInput::new(outpoint, vec![], 0, 0)], vec![output.clone()], 0, Default::default(), 0, vec![]);
    let signature = match p.signer {
        SignWith::Owner => sign_input(unsigned, entries.clone(), owner),
        SignWith::Wrong => sign_input(unsigned, entries.clone(), wrong),
        SignWith::None => vec![0u8; 65],
    };

    let new_states: Expr = vec![new_state(owner_pk, amount, p.out_for_sale, p.price)].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = input_c.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("build sigscript");
    let redeem_push = ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&input_c.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);
    let tx = Transaction::new(1, vec![TransactionInput::new(outpoint, sigscript, 0, 0)], vec![output], 0, Default::default(), 0, vec![]);
    (tx, entries)
}

// ================= 2x2 FANOUT builders =================
//
// Leader/delegate model (proven by kcc20 merge test): input 0 is the LEADER carrying the full
// covenant call args (newStates[], sigs[], witnesses[]) sized to cover BOTH inputs; input 1 is a
// DELEGATE built with empty args + is_leader:false. Each covenant input is locked by its OWN
// compiled P2SH (different owner pubkey => different script). Verify by running EVERY input.
//
// Output layout for a 2x2 claim:
//   outputs[0], outputs[1] = buyer token covenant outputs (the two new covenant UTXOs)
//   outputs[2], outputs[3] = P2PK payment outputs to the seller(s)
// So witness indices for the for-sale inputs point at 2 and/or 3.

struct FanoutClaimParams {
    // per-input for-sale flags (input 0, input 1)
    in0_for_sale: bool,
    in1_for_sale: bool,
    // listed prices per input
    price0: i64,
    price1: i64,
    // witness payment-output index per input
    witness0: u8,
    witness1: u8,
    // payment outputs to actually include: (value, pay_to_seller0?, pay_to_seller1?) — we build
    // explicit payment outputs below via `payments`
    payments: Vec<FanoutPayment>,
    // output token for-sale flags (to test claim-leaves-listing)
    out0_for_sale: bool,
    out1_for_sale: bool,
    // output token amounts (to test inflation); inputs are `amount` each
    out0_amount: i64,
    out1_amount: i64,
    // if Some(idx), input1 is signed by this owner (for mixed-state: a not-for-sale input needs a sig)
    sign_in1_with: Option<SignSel>,
}

// which payee a payment output goes to
enum Payee { Seller0, Seller1 }
struct FanoutPayment { value: u64, to: Payee }
enum SignSel { Correct, None }

#[allow(clippy::too_many_arguments)]
fn build_fanout_claim(
    source: &str,
    seller0: &Keypair,
    seller1: &Keypair,
    buyer_pk: Vec<u8>,
    amount: i64,
    p: &FanoutClaimParams,
) -> (Transaction, Vec<UtxoEntry>) {
    let s0_pk = seller0.x_only_public_key().0.serialize().to_vec();
    let s1_pk = seller1.x_only_public_key().0.serialize().to_vec();

    // Input contracts: each seller's own for-sale (or not) covenant.
    let in0_c: CompiledContract = compile_contract(source, &ctor_args(s0_pk.clone(), amount, p.in0_for_sale, p.price0), CompileOptions::default()).expect("compile in0");
    let in1_c: CompiledContract = compile_contract(source, &ctor_args(s1_pk.clone(), amount, p.in1_for_sale, p.price1), CompileOptions::default()).expect("compile in1");

    // Output token contracts: buyer-owned, flags per params.
    let out0_c: CompiledContract = compile_contract(source, &ctor_args(buyer_pk.clone(), p.out0_amount, p.out0_for_sale, 0), CompileOptions::default()).expect("compile out0");
    let out1_c: CompiledContract = compile_contract(source, &ctor_args(buyer_pk.clone(), p.out1_amount, p.out1_for_sale, 0), CompileOptions::default()).expect("compile out1");

    let op0 = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let op1 = TransactionOutpoint { transaction_id: TransactionId::from_bytes([2; 32]), index: 0 };
    let entries = vec![
        UtxoEntry::new(1_500, pay_to_script_hash_script(&in0_c.script), 0, false, Some(COV_A)),
        UtxoEntry::new(1_500, pay_to_script_hash_script(&in1_c.script), 0, false, Some(COV_A)),
    ];

    // Build outputs: two token outputs (authorizing_input 0 and 1), then payment outputs.
    let mut outputs = vec![
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&out0_c.script), covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) },
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&out1_c.script), covenant: Some(CovenantBinding { authorizing_input: 1, covenant_id: COV_A }) },
    ];
    for pay in &p.payments {
        let pk = match pay.to { Payee::Seller0 => &s0_pk, Payee::Seller1 => &s1_pk };
        outputs.push(TransactionOutput { value: pay.value, script_public_key: p2pk_spk(pk), covenant: None });
    }

    // newStates[] covers both outputs.
    let new_states: Expr = vec![
        new_state(buyer_pk.clone(), p.out0_amount, p.out0_for_sale, 0),
        new_state(buyer_pk, p.out1_amount, p.out1_for_sale, 0),
    ].into();

    // sigs[]: input0 dummy unless we later sign; input1 may be signed for mixed-state test.
    // Build an unsigned tx first if any input needs a real signature.
    let needs_sig = matches!(p.sign_in1_with, Some(SignSel::Correct));
    let sig1: Vec<u8> = if needs_sig {
        let unsigned = Transaction::new(
            1,
            vec![TransactionInput::new(op0, vec![], 0, 0), TransactionInput::new(op1, vec![], 0, 0)],
            outputs.clone(), 0, Default::default(), 0, vec![],
        );
        sign_input_idx(unsigned, entries.clone(), 0, seller1) // index 0: sigs verify in leader context
    } else {
        vec![0u8; 65]
    };
    let sigs: Expr = vec![Expr::bytes(vec![0u8; 65]), Expr::bytes(sig1)].into();

    let witnesses: Expr = Expr::bytes(vec![p.witness0, p.witness1]);

    // Leader = input 0 (in0_c), full args. Delegate = input 1 (in1_c), empty args.
    let mut leader_ss = in0_c.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("leader sigscript");
    leader_ss.extend_from_slice(&ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in0_c.script).expect("push redeem0").drain());

    let mut delegate_ss = in1_c.build_sig_script_for_covenant_decl("transfer", vec![], CovenantDeclCallOptions { is_leader: false }).expect("delegate sigscript");
    delegate_ss.extend_from_slice(&ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in1_c.script).expect("push redeem1").drain());

    let tx = Transaction::new(
        1,
        vec![TransactionInput::new(op0, leader_ss, 0, 0), TransactionInput::new(op1, delegate_ss, 0, 0)],
        outputs, 0, Default::default(), 0, vec![],
    );
    (tx, entries)
}

// 2x2 split-listing: one not-for-sale input (owner-signed leader), two outputs (one for-sale).
// This is a 1-input/2-output fanout (the split-on-list shape). Single input => single-input verify.
fn build_split_listing(source: &str, owner: &Keypair, amount: i64, list_amount: i64, keep_amount: i64, list_price: i64) -> (Transaction, Vec<UtxoEntry>) {
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    let in_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), amount, false, 0), CompileOptions::default()).expect("compile in");
    let listed_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), list_amount, true, list_price), CompileOptions::default()).expect("compile listed");
    let kept_c: CompiledContract = compile_contract(source, &ctor_args(owner_pk.clone(), keep_amount, false, 0), CompileOptions::default()).expect("compile kept");

    let op = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, pay_to_script_hash_script(&in_c.script), 0, false, Some(COV_A))];
    let outputs = vec![
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&listed_c.script), covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) },
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&kept_c.script), covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) },
    ];

    let unsigned = Transaction::new(1, vec![TransactionInput::new(op, vec![], 0, 0)], outputs.clone(), 0, Default::default(), 0, vec![]);
    let sig = sign_input(unsigned, entries.clone(), owner);

    let new_states: Expr = vec![
        new_state(owner_pk.clone(), list_amount, true, list_price),
        new_state(owner_pk, keep_amount, false, 0),
    ].into();
    let sigs: Expr = vec![Expr::bytes(sig)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut ss = in_c.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("sigscript");
    ss.extend_from_slice(&ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_c.script).expect("push redeem").drain());
    let tx = Transaction::new(1, vec![TransactionInput::new(op, ss, 0, 0)], outputs, 0, Default::default(), 0, vec![]);
    (tx, entries)
}

// ================= 3x3 FANOUT builders (bound = 3) =================
//
// Generalized N-input claim. Each input is described by a InSpec (its seller, for-sale flag,
// price, witness index, and whether it's signed — for the non-contiguous-for-sale case where a
// not-for-sale input sits between for-sale ones). Input 0 is the leader carrying full args sized
// to N; inputs 1..N are delegates. Output layout: N buyer token outputs (covenant), then the
// payment outputs the caller supplies. Compiled with maxCovIns=maxCovOuts=3.

const BOUND3: i64 = 3;

struct InSpec<'k> {
    seller: &'k Keypair,
    for_sale: bool,
    price: i64,
    witness: u8,
    signed: bool, // for a not-for-sale input that must carry a real owner sig
}

fn build_fanout3_claim(
    source: &str,
    ins: &[InSpec],
    buyer_pk: Vec<u8>,
    in_amount: i64,
    out_amounts: &[i64],
    out_for_sale: &[bool],
    payments: &[(u64, usize)], // (value, seller index in `ins` to pay)
) -> (Transaction, Vec<UtxoEntry>) {
    let n = ins.len();
    // Compile each input's locking contract (its seller's pubkey, its for-sale/price), bound 3.
    let in_cs: Vec<CompiledContract> = ins.iter().map(|s| {
        let pk = s.seller.x_only_public_key().0.serialize().to_vec();
        compile_contract(source, &ctor_args_b(pk, in_amount, s.for_sale, s.price, BOUND3, BOUND3), CompileOptions::default()).expect("compile in")
    }).collect();

    // Output token contracts: buyer-owned, bound 3.
    let out_cs: Vec<CompiledContract> = (0..n).map(|i| {
        compile_contract(source, &ctor_args_b(buyer_pk.clone(), out_amounts[i], out_for_sale[i], 0, BOUND3, BOUND3), CompileOptions::default()).expect("compile out")
    }).collect();

    let outpoints: Vec<TransactionOutpoint> = (0..n).map(|i| TransactionOutpoint { transaction_id: TransactionId::from_bytes([(i as u8) + 1; 32]), index: 0 }).collect();
    let entries: Vec<UtxoEntry> = in_cs.iter().map(|c| UtxoEntry::new(1_500, pay_to_script_hash_script(&c.script), 0, false, Some(COV_A))).collect();

    // Build outputs: N token outputs (authorizing_input i), then payment outputs.
    let mut outputs: Vec<TransactionOutput> = (0..n).map(|i| TransactionOutput {
        value: 1_000,
        script_public_key: pay_to_script_hash_script(&out_cs[i].script),
        covenant: Some(CovenantBinding { authorizing_input: i as u16, covenant_id: COV_A }),
    }).collect();
    for (val, sidx) in payments {
        let pk = ins[*sidx].seller.x_only_public_key().0.serialize().to_vec();
        outputs.push(TransactionOutput { value: *val, script_public_key: p2pk_spk(&pk), covenant: None });
    }

    // newStates[] covers all N outputs.
    let new_states: Expr = (0..n).map(|i| new_state(buyer_pk.clone(), out_amounts[i], out_for_sale[i], 0)).collect::<Vec<_>>().into();

    // sigs[]: a real sig for any input flagged signed (built over the unsigned tx), else dummy.
    let any_signed = ins.iter().any(|s| s.signed);
    let sig_bytes: Vec<Vec<u8>> = if any_signed {
        let unsigned_inputs: Vec<TransactionInput> = outpoints.iter().map(|op| TransactionInput::new(*op, vec![], 0, 0)).collect();
        let unsigned = Transaction::new(1, unsigned_inputs, outputs.clone(), 0, Default::default(), 0, vec![]);
        ins.iter().enumerate().map(|(_i, s)| {
            // NOTE: in this covenant model all signatures are verified in the LEADER's execution
            // context (input 0). The kcc20 merge test signs every input's sig at index 0, not at
            // the input's own index. Signing at index i produces the wrong sighash -> checkSig fails.
            if s.signed { sign_input_idx(unsigned.clone(), entries.clone(), 0, s.seller) } else { vec![0u8; 65] }
        }).collect()
    } else {
        ins.iter().map(|_| vec![0u8; 65]).collect()
    };
    let sigs: Expr = sig_bytes.into_iter().map(Expr::bytes).collect::<Vec<_>>().into();

    let witnesses: Expr = Expr::bytes(ins.iter().map(|s| s.witness).collect());

    // Leader = input 0, full args. Delegates = inputs 1..N, empty args.
    let mut leader_ss = in_cs[0].build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("leader ss");
    leader_ss.extend_from_slice(&ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_cs[0].script).expect("redeem0").drain());

    let mut tx_inputs = vec![TransactionInput::new(outpoints[0], leader_ss, 0, 0)];
    for i in 1..n {
        let mut d_ss = in_cs[i].build_sig_script_for_covenant_decl("transfer", vec![], CovenantDeclCallOptions { is_leader: false }).expect("delegate ss");
        d_ss.extend_from_slice(&ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_cs[i].script).expect("redeemN").drain());
        tx_inputs.push(TransactionInput::new(outpoints[i], d_ss, 0, 0));
    }

    let tx = Transaction::new(1, tx_inputs, outputs, 0, Default::default(), 0, vec![]);
    (tx, entries)
}

// 3x3 split-listing: one not-for-sale input (bound 3), owner-signed, three outputs (one listed, two kept).
fn build_split_listing3(source: &str, owner: &Keypair, amount: i64, a: i64, b: i64, c: i64, list_price: i64) -> (Transaction, Vec<UtxoEntry>) {
    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    let in_c: CompiledContract = compile_contract(source, &ctor_args_b(owner_pk.clone(), amount, false, 0, BOUND3, BOUND3), CompileOptions::default()).expect("in");
    let c0: CompiledContract = compile_contract(source, &ctor_args_b(owner_pk.clone(), a, true, list_price, BOUND3, BOUND3), CompileOptions::default()).expect("c0");
    let c1: CompiledContract = compile_contract(source, &ctor_args_b(owner_pk.clone(), b, false, 0, BOUND3, BOUND3), CompileOptions::default()).expect("c1");
    let c2: CompiledContract = compile_contract(source, &ctor_args_b(owner_pk.clone(), c, false, 0, BOUND3, BOUND3), CompileOptions::default()).expect("c2");

    let op = TransactionOutpoint { transaction_id: TransactionId::from_bytes([1; 32]), index: 0 };
    let entries = vec![UtxoEntry::new(1_500, pay_to_script_hash_script(&in_c.script), 0, false, Some(COV_A))];
    let outputs = vec![
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&c0.script), covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) },
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&c1.script), covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) },
        TransactionOutput { value: 1_000, script_public_key: pay_to_script_hash_script(&c2.script), covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: COV_A }) },
    ];
    let unsigned = Transaction::new(1, vec![TransactionInput::new(op, vec![], 0, 0)], outputs.clone(), 0, Default::default(), 0, vec![]);
    let sig = sign_input(unsigned, entries.clone(), owner);
    let new_states: Expr = vec![
        new_state(owner_pk.clone(), a, true, list_price),
        new_state(owner_pk.clone(), b, false, 0),
        new_state(owner_pk, c, false, 0),
    ].into();
    let sigs: Expr = vec![Expr::bytes(sig)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut ss = in_c.build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true }).expect("ss");
    ss.extend_from_slice(&ScriptBuilder::with_flags(EngineFlags { covenants_enabled: true, sigop_script_units: 0.into() }).add_data(&in_c.script).expect("redeem").drain());
    let tx = Transaction::new(1, vec![TransactionInput::new(op, ss, 0, 0)], outputs, 0, Default::default(), 0, vec![]);
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

    println!("== CLAIM path (payment-authorized, unsigned) — 1x1 ==");
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

    println!("== LISTING path (owner-signed) — 1x1 ==");
    let ml = |p: ListParams| build_listing(&source, &seller, &wrong, amount, &p);
    let (tx, e) = ml(ListParams { out_for_sale: true, price, signer: SignWith::Owner });
    run("L-CONTROL", true, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = ml(ListParams { out_for_sale: true, price, signer: SignWith::None });
    run("L1 nosig  ", false, vm_verify(&tx, e), &mut all_ok);
    let (tx, e) = ml(ListParams { out_for_sale: true, price, signer: SignWith::Wrong });
    run("L2 wrong  ", false, vm_verify(&tx, e), &mut all_ok);

    // ================= 2x2 FANOUT =================
    println!("== 2x2 FANOUT ==");
    let seller0 = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let seller1 = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let big_pay = 100_000_000u64; // well above price, dodges KIP-9 dust at the VM level it's irrelevant but keep realistic

    // F-CONTROL-claim: two for-sale inputs, distinct sellers, two distinct payments (out idx 2 -> s0, idx 3 -> s1).
    let p = FanoutClaimParams {
        in0_for_sale: true, in1_for_sale: true, price0: price, price1: price,
        witness0: 2, witness1: 3,
        payments: vec![FanoutPayment { value: big_pay, to: Payee::Seller0 }, FanoutPayment { value: big_pay, to: Payee::Seller1 }],
        out0_for_sale: false, out1_for_sale: false, out0_amount: amount, out1_amount: amount,
        sign_in1_with: None,
    };
    let (tx, e) = build_fanout_claim(&source, &seller0, &seller1, buyer_pk.clone(), amount, &p);
    run("F-CTRL-claim", true, vm_verify_all(&tx, e, 2), &mut all_ok);

    // F-CONTROL-split: single input, two outputs (one listed, one kept). 1-input verify.
    let (tx, e) = build_split_listing(&source, &seller0, amount, 400, 600, price);
    run("F-CTRL-split", true, vm_verify(&tx, e), &mut all_ok);

    // F1 shared-output double-claim: SAME seller both inputs, witnesses=[2,2], ONE payment at out idx 2.
    // (Both token outputs go to buyer; only one payment output exists.)
    let p = FanoutClaimParams {
        in0_for_sale: true, in1_for_sale: true, price0: price, price1: price,
        witness0: 2, witness1: 2,
        payments: vec![FanoutPayment { value: big_pay, to: Payee::Seller0 }],
        out0_for_sale: false, out1_for_sale: false, out0_amount: amount, out1_amount: amount,
        sign_in1_with: None,
    };
    // both inputs same seller: pass seller0 as both
    let (tx, e) = build_fanout_claim(&source, &seller0, &seller0, buyer_pk.clone(), amount, &p);
    run("F1 dblclaim ", false, vm_verify_all(&tx, e, 2), &mut all_ok);

    // F2 cross-wired witness: distinct sellers, both witnesses point at out idx 2 (pays seller0 only).
    // seller1's input is validated against seller0's payment => seller1 robbed.
    let p = FanoutClaimParams {
        in0_for_sale: true, in1_for_sale: true, price0: price, price1: price,
        witness0: 2, witness1: 2,
        payments: vec![FanoutPayment { value: big_pay, to: Payee::Seller0 }],
        out0_for_sale: false, out1_for_sale: false, out0_amount: amount, out1_amount: amount,
        sign_in1_with: None,
    };
    let (tx, e) = build_fanout_claim(&source, &seller0, &seller1, buyer_pk.clone(), amount, &p);
    run("F2 crosswire", false, vm_verify_all(&tx, e, 2), &mut all_ok);

    // F3 fanout inflation: 2-in (1000+1000), 2-out (1000+1500). totalOut > totalIn.
    let p = FanoutClaimParams {
        in0_for_sale: true, in1_for_sale: true, price0: price, price1: price,
        witness0: 2, witness1: 3,
        payments: vec![FanoutPayment { value: big_pay, to: Payee::Seller0 }, FanoutPayment { value: big_pay, to: Payee::Seller1 }],
        out0_for_sale: false, out1_for_sale: false, out0_amount: amount, out1_amount: amount + 500,
        sign_in1_with: None,
    };
    let (tx, e) = build_fanout_claim(&source, &seller0, &seller1, buyer_pk.clone(), amount, &p);
    run("F3 inflate  ", false, vm_verify_all(&tx, e, 2), &mut all_ok);

    // F4 claim-leaves-listing: 2-in claim, output 0 stays for-sale.
    let p = FanoutClaimParams {
        in0_for_sale: true, in1_for_sale: true, price0: price, price1: price,
        witness0: 2, witness1: 3,
        payments: vec![FanoutPayment { value: big_pay, to: Payee::Seller0 }, FanoutPayment { value: big_pay, to: Payee::Seller1 }],
        out0_for_sale: true, out1_for_sale: false, out0_amount: amount, out1_amount: amount,
        sign_in1_with: None,
    };
    let (tx, e) = build_fanout_claim(&source, &seller0, &seller1, buyer_pk.clone(), amount, &p);
    run("F4 leavelist", false, vm_verify_all(&tx, e, 2), &mut all_ok);

    // F5 mixed-state auth bypass: input0 for-sale (paid via out idx 2), input1 NOT for-sale & unsigned.
    let p = FanoutClaimParams {
        in0_for_sale: true, in1_for_sale: false, price0: price, price1: 0,
        witness0: 2, witness1: 0,
        payments: vec![FanoutPayment { value: big_pay, to: Payee::Seller0 }],
        out0_for_sale: false, out1_for_sale: false, out0_amount: amount, out1_amount: amount,
        sign_in1_with: None, // input1 NOT signed -> must fail (not-for-sale needs owner sig)
    };
    let (tx, e) = build_fanout_claim(&source, &seller0, &seller1, buyer_pk.clone(), amount, &p);
    run("F5 mixedauth", false, vm_verify_all(&tx, e, 2), &mut all_ok);

    // ================= 3x3 FANOUT (bound = 3) =================
    println!("== 3x3 FANOUT ==");
    let s0 = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let s1 = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let s2 = Keypair::new(secp256k1::SECP256K1, &mut secp256k1::rand::thread_rng());
    let bp = big_pay;
    let amt3 = [amount, amount, amount];
    let nofs3 = [false, false, false];

    // G-CTRL-claim: 3 for-sale inputs, witnesses [3,4,5], 3 distinct payments. PASS.
    let ins = [
        InSpec { seller: &s0, for_sale: true, price, witness: 3, signed: false },
        InSpec { seller: &s1, for_sale: true, price, witness: 4, signed: false },
        InSpec { seller: &s2, for_sale: true, price, witness: 5, signed: false },
    ];
    let pays = [(bp, 0usize), (bp, 1usize), (bp, 2usize)];
    let (tx, e) = build_fanout3_claim(&source, &ins, buyer_pk.clone(), amount, &amt3, &nofs3, &pays);
    run("G-CTRL-claim", true, vm_verify_all(&tx, e, 3), &mut all_ok);

    // G-CTRL-split: 1 input -> 3 outputs (one listed, two kept), owner-signed. PASS.
    let (tx, e) = build_split_listing3(&source, &s0, amount, 300, 300, 400, price);
    run("G-CTRL-split", true, vm_verify(&tx, e), &mut all_ok);

    // G1 triple-share: witnesses [3,3,3], one payment. FAIL (2nd input: 3 not > 3).
    let ins = [
        InSpec { seller: &s0, for_sale: true, price, witness: 3, signed: false },
        InSpec { seller: &s0, for_sale: true, price, witness: 3, signed: false },
        InSpec { seller: &s0, for_sale: true, price, witness: 3, signed: false },
    ];
    let pays = [(bp, 0usize)];
    let (tx, e) = build_fanout3_claim(&source, &ins, buyer_pk.clone(), amount, &amt3, &nofs3, &pays);
    run("G1 tripleshr", false, vm_verify_all(&tx, e, 3), &mut all_ok);

    // G2 middle-collision: witnesses [3,4,4]. FAIL (3rd collides with 2nd).
    let ins = [
        InSpec { seller: &s0, for_sale: true, price, witness: 3, signed: false },
        InSpec { seller: &s1, for_sale: true, price, witness: 4, signed: false },
        InSpec { seller: &s2, for_sale: true, price, witness: 4, signed: false },
    ];
    let pays = [(bp, 0usize), (bp, 1usize)];
    let (tx, e) = build_fanout3_claim(&source, &ins, buyer_pk.clone(), amount, &amt3, &nofs3, &pays);
    run("G2 midcollis", false, vm_verify_all(&tx, e, 3), &mut all_ok);

    // G3 out-of-order: witnesses [5,3,4] -- all distinct but descending-then-up. FAIL (3 not > 5).
    // Documents that strict-increasing is MORE restrictive than minimal: it forbids distinct-but-
    // unordered witnesses (which are not actually a double-claim). Safe, but a known restriction;
    // the per-output-accounting mainnet version would accept this.
    let ins = [
        InSpec { seller: &s0, for_sale: true, price, witness: 5, signed: false },
        InSpec { seller: &s1, for_sale: true, price, witness: 3, signed: false },
        InSpec { seller: &s2, for_sale: true, price, witness: 4, signed: false },
    ];
    let pays = [(bp, 0usize), (bp, 1usize), (bp, 2usize)];
    let (tx, e) = build_fanout3_claim(&source, &ins, buyer_pk.clone(), amount, &amt3, &nofs3, &pays);
    run("G3 outoforder", false, vm_verify_all(&tx, e, 3), &mut all_ok);

    // G4 non-contiguous for-sale: input0 for-sale (w=3), input1 NOT for-sale & SIGNED, input2 for-sale (w=4).
    // PASS: accumulator must skip the signed middle input and still accept 3 < 4.
    let ins = [
        InSpec { seller: &s0, for_sale: true, price, witness: 3, signed: false },
        InSpec { seller: &s1, for_sale: false, price: 0, witness: 0, signed: true },
        InSpec { seller: &s2, for_sale: true, price, witness: 4, signed: false },
    ];
    let pays = [(bp, 0usize), (bp, 2usize)];
    let (tx, e) = build_fanout3_claim(&source, &ins, buyer_pk.clone(), amount, &amt3, &nofs3, &pays);
    run("G4 noncontig", true, vm_verify_all(&tx, e, 3), &mut all_ok);

    println!();
    if all_ok {
        println!("[RESULT] SALE+LISTING+FANOUT GATE PROVEN: 1x1 + 2x2 + 3x3 all behave; strict-increasing scales to 3x3.");
    } else {
        println!("[RESULT] *** GATE NOT PROVEN — see UNEXPECTED/HOLE lines. DO NOT deploy. ***");
    }
}
