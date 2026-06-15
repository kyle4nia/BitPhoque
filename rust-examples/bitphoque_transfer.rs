// BitPhoque — TRANSFER (handoff) harness (TN10 practice).
//
// Spends the live genesis covenant UTXO and creates ONE new covenant UTXO
// owned by a fresh recipient key, moving the full 42_069 supply (a "handoff").
// Uses the leader entrypoint of the transfer ABI.
//
// Mechanics (all read from kcc20_tests.rs + silverscript-lang public API):
//  - recompile the contract with the NEW owner -> recipient redeem script + P2SH
//  - find the live genesis covenant UTXO at the CURRENT owner's token P2SH
//  - build unsigned tx: in = genesis cov utxo, out = recipient P2SH (bound, same cov_id)
//  - sign input 0 with the CURRENT owner key (schnorr, SIG_HASH_ALL, +type byte)
//  - leader sigscript = build_sig_script_for_covenant_decl("transfer",
//        [newStates, sigs, witnesses], is_leader=true) ++ push(redeem)
//  - dry-run by default; --go to broadcast
//
// TN10 PRACTICE ONLY. Mainnet = detached air-gapped signing (see checklist).

use std::fs;

use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::{
    constants::TX_VERSION_TOCCATA,
    hashing::sighash::{calc_schnorr_signature_hash, SigHashReusedValuesUnsync},
    hashing::sighash_type::SIG_HASH_ALL,
    subnets::SUBNETWORK_ID_NATIVE,
    tx::{
        CovenantBinding, MutableTransaction, Transaction, TransactionInput, TransactionOutpoint,
        TransactionOutput, TxInputMass, UtxoEntry,
    },
};
use kaspa_grpc_client::GrpcClient;
use kaspa_hashes::Hash;
use kaspa_notify::subscription::context::SubscriptionContext;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_txscript::{pay_to_script_hash_script, extract_script_pub_key_address, script_builder::ScriptBuilder};
use secp256k1::{rand, Keypair};
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, struct_object, CompileOptions, CovenantDeclCallOptions};

const NODE: &str = "192.168.0.206:16210";
const KEY_FILE: &str = r"C:\bitphoque\bitphoque_owner.key";
const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque.sil";
const COVENANT_ID_HEX: &str = "69262e04b63d1ea1008fd33f8aff38c08780056b40111d0db01a306ef11a766c";

const SUPPLY: i64 = 42_069;
const FEE: u64 = 200_000;
const MAX_COV_INS: i64 = 2;
const MAX_COV_OUTS: i64 = 2;

// Build the 6-arg ctor as Expr values (matches genesis ctor, pubkey-owned, fixed supply).
fn ctor_args(owner: Vec<u8>) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner),
        Expr::int(SUPPLY),
        Expr::byte(0),       // identifierType 0x00 = pubkey
        Expr::bool(false),   // isMinter = false
        Expr::int(MAX_COV_INS),
        Expr::int(MAX_COV_OUTS),
    ]
}

// One new-state struct: (ownerIdentifier, identifierType=0, amount, isMinter=false).
fn new_state(owner: Vec<u8>, amount: i64) -> Expr<'static> {
    struct_object(vec![
        ("ownerIdentifier", Expr::bytes(owner)),
        ("identifierType", Expr::byte(0)),
        ("amount", Expr::int(amount)),
        ("isMinter", Expr::bool(false)),
    ])
}

fn load_current_keypair() -> Keypair {
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

#[tokio::main]
async fn main() {
    let go = std::env::args().any(|a| a == "--go");

    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    let current = load_current_keypair();
    let current_pk = current.x_only_public_key().0.serialize().to_vec();

    // Recompile the CURRENT contract (to derive the token P2SH we're spending from).
    let current_compiled =
        compile_contract(&source, &ctor_args(current_pk.clone()), CompileOptions::default())
            .expect("compile current");
    let current_p2sh = pay_to_script_hash_script(&current_compiled.script);

    // Fresh recipient key (the handoff destination). TN10 practice: print its privkey.
    let recipient = Keypair::new(secp256k1::SECP256K1, &mut rand::thread_rng());
    let recipient_pk = recipient.x_only_public_key().0.serialize().to_vec();
    let recipient_sk = recipient.secret_key().secret_bytes();
    let mut sk_hex = vec![0u8; 64];
    faster_hex::hex_encode(&recipient_sk, &mut sk_hex).unwrap();
    let recipient_addr = Address::new(Prefix::Testnet, Version::PubKey, &recipient_pk);
    println!("[transfer] recipient pubkey: {}", faster_hex::hex_string(&recipient_pk));
    println!("[transfer] recipient privkey: {}", String::from_utf8(sk_hex).unwrap());
    println!("[transfer] recipient P2PK addr: {recipient_addr}");

    // Recompile with the RECIPIENT owner -> the output's redeem script + P2SH.
    let recipient_compiled =
        compile_contract(&source, &ctor_args(recipient_pk.clone()), CompileOptions::default())
            .expect("compile recipient");
    let recipient_p2sh = pay_to_script_hash_script(&recipient_compiled.script);

    // Connect, find the live genesis covenant UTXO at the current token P2SH.
    let subscription_context = SubscriptionContext::new();
    println!("[transfer] connecting to grpc://{NODE}");
    let rpc = GrpcClient::connect_with_args(
        NotificationMode::Direct,
        format!("grpc://{NODE}"),
        Some(subscription_context.clone()),
        true, None, false, Some(500_000), Default::default(),
    ).await.expect("connect failed");
    println!("[transfer] connected.");

    let token_addr = extract_script_pub_key_address(&current_p2sh, Prefix::Testnet).expect("p2sh address");
    let resp = rpc.get_utxos_by_addresses(vec![token_addr]).await.expect("get_utxos failed");
    if resp.is_empty() {
        println!("[transfer] no covenant UTXO at token P2SH. aborting.");
        return;
    }
    let entry = &resp[0];
    let outpoint = TransactionOutpoint::from(entry.outpoint.clone());
    let in_value = entry.utxo_entry.amount;
    println!("[transfer] spending genesis cov utxo {}:{}  value {}", outpoint.transaction_id, outpoint.index, in_value);

    let cov_id = covenant_id();
    let in_utxo = UtxoEntry::new(in_value, current_p2sh.clone(), 0, false, Some(cov_id));

    // Output: recipient P2SH, value = in - fee, bound to same covenant_id, authorized by input 0.
    let out_value = in_value - FEE;
    let output = TransactionOutput {
        value: out_value,
        script_public_key: recipient_p2sh,
        covenant: Some(CovenantBinding { authorizing_input: 0, covenant_id: cov_id }),
    };

    // Unsigned tx (empty sigscript) to sign over.
    let in_unsigned = TransactionInput { previous_outpoint: outpoint, signature_script: vec![], sequence: 0, mass: TxInputMass::SigopCount(1.into()) };
    let unsigned = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_unsigned], vec![output.clone()], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);

    // Sign input 0 with the CURRENT owner key.
    let mtx = MutableTransaction::with_entries(unsigned, vec![in_utxo.clone()]);
    let reused = SigHashReusedValuesUnsync::new();
    let sig_hash = calc_schnorr_signature_hash(&mtx.as_verifiable(), 0, SIG_HASH_ALL, &reused);
    let msg = secp256k1::Message::from_digest_slice(sig_hash.as_bytes().as_slice()).expect("sighash msg");
    let sig = current.sign_schnorr(msg);
    let mut signature = sig.as_ref().to_vec();
    signature.push(SIG_HASH_ALL.to_u8());

    // Leader sigscript: args + pushed redeem script.
    let new_states: Expr = vec![new_state(recipient_pk.clone(), SUPPLY)].into();
    let sigs: Expr = vec![Expr::bytes(signature)].into();
    let witnesses: Expr = Expr::bytes(vec![0u8]);
    let mut sigscript = current_compiled
        .build_sig_script_for_covenant_decl("transfer", vec![new_states, sigs, witnesses], CovenantDeclCallOptions { is_leader: true })
        .expect("build leader sigscript");
    let redeem_push = ScriptBuilder::new().add_data(&current_compiled.script).expect("push redeem").drain();
    sigscript.extend_from_slice(&redeem_push);

    // Rebuild tx with the real sigscript.
    let in_signed = TransactionInput { previous_outpoint: outpoint, signature_script: sigscript, sequence: 0, mass: TxInputMass::SigopCount(1.into()) };
    let mut tx = Transaction::new_non_finalized(TX_VERSION_TOCCATA, vec![in_signed], vec![output], 0, SUBNETWORK_ID_NATIVE, 0, vec![]);
    tx.finalize();

    println!("[transfer] handoff tx id: {}", tx.id());
    println!("[transfer] moving {SUPPLY} phoques -> recipient, output value {out_value} sompi (fee {FEE})");

    if !go {
        println!();
        println!("[transfer] DRY RUN — not submitted. Re-run with --go to broadcast.");
        return;
    }

    println!("[transfer] SUBMITTING...");
    match rpc.submit_transaction((&tx).into(), false).await {
        Ok(resp) => println!("[transfer] ACCEPTED. node response: {resp:?}"),
        Err(e) => println!("[transfer] REJECTED: {e}"),
    }
}
