// BitPhoque — owner keypair generator (TN10 PRACTICE KEY ONLY).
//
// Generates a fresh secp256k1 keypair and prints:
//   - privkey hex (32 bytes)      -> hold locally; the harness signs with this
//   - x-only pubkey hex (32 bytes) -> this is `genesisPk` for the BitPhoque ctor
//   - TN10 P2PK address            -> if you fund genesis from this same key
//
// SAFETY (TN10 vs MAINNET): printing the privkey to the terminal is fine for a
// throwaway testnet key with no real value. For MAINNET, the owner key controls
// the ENTIRE token supply and must be generated air-gapped and NEVER printed.
// This tool is the testnet-practice version of that step, not the mainnet one.

use kaspa_addresses::{Address, Prefix, Version};
use secp256k1::{rand, Keypair, SECP256K1};

fn main() {
    // Fresh random keypair.
    let keypair = Keypair::new(SECP256K1, &mut rand::thread_rng());

    // Private key (32 bytes) -> hex.
    let sk_bytes = keypair.secret_key().secret_bytes();
    let mut sk_hex = vec![0u8; 64];
    faster_hex::hex_encode(&sk_bytes, &mut sk_hex).expect("encode sk");
    let sk_hex = String::from_utf8(sk_hex).expect("sk utf8");

    // x-only pubkey (32 bytes) -> hex. THIS is genesisPk for the ctor.
    let (xonly, _parity) = keypair.x_only_public_key();
    let xonly_bytes = xonly.serialize();
    let mut pk_hex = vec![0u8; 64];
    faster_hex::hex_encode(&xonly_bytes, &mut pk_hex).expect("encode pk");
    let pk_hex = String::from_utf8(pk_hex).expect("pk utf8");

    // TN10 P2PK address from the x-only pubkey (testnet prefix).
    let addr = Address::new(Prefix::Testnet, Version::PubKey, &xonly_bytes);

    println!("=== BitPhoque owner keypair (TN10 practice) ===");
    println!("privkey hex : {sk_hex}");
    println!("xonly pubkey: {pk_hex}");
    println!("P2PK addr   : {addr}");
    println!();
    println!("Next: put the xonly pubkey into the BitPhoque ctor as genesisPk (32 byte entries),");
    println!("recompile bitphoque.sil, and use the privkey hex in the genesis harness to sign.");
}
