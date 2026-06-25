// BitPhoque — SCRIPT DIFF (read-only, no node, no keys, no tx).
// Compiles the v3 covenant for several ctor-arg sets and dumps each compiled
// script as hex + length. Purpose: decide whether the browser can build the
// P2SH redeem script by splicing operands into a captured template (Route B1)
// or whether CScriptNum width-variation forces a real compiler in WASM (B2/B3).
//
// We print, for each case: byte length + full hex. Diff them by eye / with fc.
//   A vs B  (only forSale bool flips)      -> expect a single-byte diff, same length
//   A vs C  (amount/price cross 127->128)  -> if length or offsets shift, splice is width-sensitive
use std::fs;
use kaspa_txscript::pay_to_script_hash_script;
use silverscript_lang::ast::Expr;
use silverscript_lang::compiler::{compile_contract, CompileOptions};

const SIL_FILE: &str = r"C:\bitphoque\covenants\bitphoque_sale_v3.sil";
const OWNER_HEX: &str = "8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936";
const MAX_COV_INS: i64 = 3;
const MAX_COV_OUTS: i64 = 3;

fn owner_bytes() -> Vec<u8> {
    let mut pk = [0u8; 32];
    faster_hex::hex_decode(OWNER_HEX.trim().as_bytes(), &mut pk).expect("bad owner hex");
    pk.to_vec()
}

fn ctor_args(owner: Vec<u8>, amount: i64, for_sale: bool, price: i64) -> Vec<Expr<'static>> {
    vec![
        Expr::bytes(owner), Expr::int(amount), Expr::byte(0), Expr::bool(false),
        Expr::bool(for_sale), Expr::int(price), Expr::int(MAX_COV_INS), Expr::int(MAX_COV_OUTS),
    ]
}

fn dump(source: &str, label: &str, amount: i64, for_sale: bool, price: i64) {
    let compiled = compile_contract(source, &ctor_args(owner_bytes(), amount, for_sale, price), CompileOptions::default()).expect("compile");
    let script = &compiled.script;
    let p2sh = pay_to_script_hash_script(script);
    let mut hex = vec![0u8; script.len() * 2];
    faster_hex::hex_encode(script, &mut hex).unwrap();
    let mut p2sh_hex = vec![0u8; p2sh.script().len() * 2];
    faster_hex::hex_encode(p2sh.script(), &mut p2sh_hex).unwrap();
    println!("[{label}] amount={amount} forSale={for_sale} price={price}");
    println!("  script_len = {}", script.len());
    println!("  script_hex = {}", String::from_utf8(hex).unwrap());
    println!("  p2sh_spk   = {}", String::from_utf8(p2sh_hex).unwrap());
    println!();
}

fn main() {
    let source = fs::read_to_string(SIL_FILE).expect("read sil");
    println!("=== BitPhoque v3 script diff — owner buyer1, MAX_INS/OUTS {MAX_COV_INS}/{MAX_COV_OUTS} ===\n");
    dump(&source, "A live-3000", 3000, true, 500_000);
    dump(&source, "B flip-bool", 3000, false, 500_000);
    dump(&source, "C boundary ", 127, true, 128);
    dump(&source, "D big-vals ", 42069, true, 1_000_000_000);
    println!("READ: compare A vs B (bool only) and A vs C/D (int widths).");
    println!("  same length + single localized diff -> Route B1 (template splice) viable.");
    println!("  length/offset shifts with value     -> CScriptNum width-sensitive; need B2/B3 WASM.");
}
