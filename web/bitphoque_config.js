/**
 *  bitphoque_config.js — network configuration (the ONE file that differs
 *  between testnet and mainnet deployments).
 *
 *  Every other module imports from here instead of hardcoding network,
 *  covenant_id, or node URL.  To switch networks: change these values
 *  and redeploy.  Nothing else should need editing.
 */

// ── Network ────────────────────────────────────────────────────────
//  'kaspatest'  →  TN10 testnet
//  'kaspa'      →  mainnet
export const NETWORK = 'kaspatest';

// ── Covenant identity ──────────────────────────────────────────────
//  Full 64-char hex  (used in tx building, descriptor codec)
//  Short 8-char prefix  (used in UTXO filtering)
//  Both change on every re-genesis (v4 compile, mainnet deploy, etc.)
export const COVENANT_ID_HEX =
  '3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458';
export const COVENANT_ID_SHORT = '3baed5a3';

// ── Node URL ───────────────────────────────────────────────────────
//  Public endpoint — used when wallet isn't on localhost or LAN.
//  LAN / localhost overrides stay in wallet.html (runtime detection).
export const WRPC_URL_PUBLIC = 'wss://node.outlawpolitics.org';

// ── Fee floors (sompi) ─────────────────────────────────────────────
//  Same for both networks (driven by script size + KIP-9, not network).
export const CLAIM_FEE     = 900_000n;
export const CLAIM_PAYMENT = 100_000_000n;

// ── HD derivation ──────────────────────────────────────────────────
//  Coin type 111111 for Kaspa.  Same path both networks.
export const HD_PATH = "m/44'/111111'/0'/0/0";
