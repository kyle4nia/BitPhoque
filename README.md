# BitPhoque

A covenant-based fungible token on Kaspa. Native Layer 1.

## What this is

Fixed-supply token built on SilverScript covenants (post-Toccata, `binding=cov` N-to-M plumbing). 42,069 phoques. Per-UTXO balances, conservation enforced at the contract layer.

Phonetic play on "bit f**ks to give." Seal mascot. The name is the vibe. For additional cultural reference it is highly recommended you see: https://youtu.be/OMuHtTGqu6o?si=ntgjsBO1HkRn2Ivn

## How it works

Each parcel of phoques lives at its own P2SH address derived from the covenant script, the owner's public key, the amount, and the sale state. The sale model is built into the contract: an owner lists a parcel at a price, a buyer claims it by including the correct payment output. The contract verifies the payment.

**Contract source:** `covenants/bitphoque_sale_v3.sil` (live on TN10), `covenants/bitphoque_sale_v4.sil` (mainnet prep, tighter guards).

## The wallet

`web/wallet.html` -- runs entirely in the browser. BIP-39 seed phrase, AES-GCM encrypted persistence, HD key derivation. All crypto libraries vendored locally. Talks to a kaspad node over JSON wRPC (WebSocket).

Handles send, receive, split, merge, list-for-sale, and buy. Builds, signs, and submits covenant transactions directly from the browser.

Live at [bitphoque.org](https://bitphoque.org/).

## The CLI

`rust-examples/bitphoque_cli.rs` -- nine subcommands: import, balance, list, split, merge, send, receive, partial-list, claim. Built against rusty-kaspa with local `[patch]` redirects. Reference implementation for the transaction formats.

## Current state

Live on Kaspa TN10. Mainnet blocked on the SilverScript V1 compiler release (entrypoint arg-length validation is a no-op in pre-stable). Contract logic has been through three independent review passes and an internal audit. When V1 ships, the mainnet ceremony is documented and ready to execute.

## What is novel (maybe)

I do not know what else exists in this space yet. But these were hard-won and might be useful to someone:

- **Full browser-native covenant tx building.** Sighash computation, schnorr signing, and ABI sigscript construction in vanilla JS. `bitphoque_sighash.js`, `bitphoque_sigscript.js`, `bitphoque_tx.js`.
- **Per-UTXO P2SH derivation from covenant state.** Template splice at fixed byte offsets in the compiled script. `bitphoque_p2sh.js`.
- **Parcel descriptor codec.** Fixed-width 118-byte descriptors for off-chain parcel transfer coordination. `bitphoque_descriptor.js`.
- **JSON wRPC from the browser.** Plain WebSocket to kaspad's `--rpclisten-json` endpoint. This was undocumented when I figured it out.

## The bigger picture

I wrote a book called *Revolution: A Practical Guide* about transparent governance and systems that adjust themselves toward their stated goals without a privileged operator doing the adjusting. I call this property "self-erecting." BitPhoque is a concrete instance of that idea applied to a token. The Oracle Protocol (separate repo) takes the same idea further -- reputation as a UTXO, honest behavior incentivized at the protocol layer. BitPhoque is the proving ground for the covenant toolchain that both projects share.

## File structure

```
covenants/     Covenant source (.sil) and compiled artifacts (.json)
rust-examples/ CLI and harness source (.rs)
web/           Wallet, JS modules, vendored crypto libs, bulletin board service
```

## Building

The Rust harnesses build against a local checkout of rusty-kaspa with `[patch]` overrides in the workspace Cargo.toml. The web wallet is static files -- open `wallet.html` in a browser pointed at a kaspad node with `--rpclisten-json`.

---

*Solo build. In search of like-minded collaborators.*
