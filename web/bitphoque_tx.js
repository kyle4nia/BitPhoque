/* ============================================================================
   BitPhoque — transaction builder (send / split / list)
   ----------------------------------------------------------------------------
   Wires the proven pieces into a complete covenant transaction pipeline:
     bitphoque_sighash.js  → sighash computation (proven vs Rust test vectors)
     bitphoque_sigscript.js → ABI sigscript (proven vs CLI dry-run bytes)
     bitphoque_p2sh.js     → P2SH derivation (proven vs on-chain parcels)
     @noble/curves schnorr  → signing (proven via key derivation + KasWare)

   SCOPE: owner-signed, leader ABI transactions (send, split, list).
   Claim (2-input, payment-authorized) is a separate path — not here yet.
   ============================================================================ */
import { calcSchnorrSignatureHash, SIG_HASH_ALL, hexToBytes, bytesToHex } from './bitphoque_sighash.js';
import { buildSigScript } from './bitphoque_sigscript.js';
import { deriveRedeemScript, deriveScriptHashHex } from './bitphoque_p2sh.js';
import { schnorr } from './vendor/noble-curves/secp256k1.js';

// --- constants ---------------------------------------------------------------
const FEE = 700_000n;
const MIN_OUT_VALUE = 1_000_000n;
const COMPUTE_BUDGET = 24;                 // checked_covering_script_units(240000)
const COVENANT_ID_HEX = '3baed5a39ed95e6aa62a0d8d46aae39a8d832d4f900e5a6a351dca4fbc326458';
const COVENANT_ID_BYTES = hexToBytes(COVENANT_ID_HEX);
const SUBNETWORK_NATIVE_HEX = '0000000000000000000000000000000000000000';

// --- helpers -----------------------------------------------------------------
// scriptPubKey bytes: OP_BLAKE2B OP_DATA32 <hash> OP_EQUAL = aa20<hash>87
function makeSpk(hashHex) { return hexToBytes('aa20' + hashHex + '87'); }

// JSON wire format for scriptPublicKey: u16 LE version hex + script hex
function spkJsonHex(version, spkBytes) {
  return version.toString(16).padStart(4, '0') + bytesToHex(spkBytes);
}

// --- build a signed owner-signed leader-ABI tx -------------------------------
//
// params = {
//   inputTxId:     string (64-char hex)
//   inputIndex:    number
//   inputValue:    BigInt (sompi)
//   inputOwner:    string (64-char hex, x-only)
//   inputAmount:   BigInt (phoques — covenant amount)
//   inputForSale:  bool
//   inputPrice:    BigInt (sompi)
//   newStates:     [{ owner(hex), amount(BigInt), forSale(bool), price(BigInt) }]
//   privkeyHex:    string (64-char hex, 32-byte private key)
// }
//
// Returns { txJson, sigHashHex, sigHex, txSummary }
//
function buildOwnerSignedTx(params) {
  const {
    inputTxId, inputIndex, inputValue,
    inputOwner, inputAmount, inputForSale, inputPrice,
    newStates, privkeyHex,
  } = params;

  // --- derive input UTXO scriptPubKey (for sighash) --------------------------
  const inputSpkHash = deriveScriptHashHex(inputOwner, inputAmount, !!inputForSale, inputPrice || 0n);
  const inputSpk = makeSpk(inputSpkHash);

  // --- derive output scriptPubKeys -------------------------------------------
  const spendable = BigInt(inputValue) - FEE;
  if (spendable <= 0n) throw new Error('UTXO value too small for fee');

  const totalOutAmount = newStates.reduce((s, st) => s + BigInt(st.amount), 0n);
  if (totalOutAmount !== BigInt(inputAmount))
    throw new Error('conservation violated: input ' + inputAmount + ' != output ' + totalOutAmount);

  // Proportional value split (matches CLI: integer division, remainder to last)
  const outputValues = [];
  let remaining = spendable;
  for (let i = 0; i < newStates.length; i++) {
    if (i === newStates.length - 1) {
      outputValues.push(remaining);
    } else {
      const v = (spendable * BigInt(newStates[i].amount)) / BigInt(inputAmount);
      outputValues.push(v);
      remaining -= v;
    }
  }

  // Floor check
  for (let i = 0; i < outputValues.length; i++) {
    if (outputValues[i] < MIN_OUT_VALUE)
      throw new Error('output ' + i + ' value ' + outputValues[i] + ' below MIN_OUT_VALUE');
  }

  // Build output objects for sighash
  const outputs = newStates.map((st, i) => {
    const hashHex = deriveScriptHashHex(st.owner, BigInt(st.amount), !!st.forSale, BigInt(st.price || 0));
    return {
      value: outputValues[i],
      scriptPubKeyVersion: 0,
      scriptPubKey: makeSpk(hashHex),
      covenant: { authorizingInput: 0, covenantId: COVENANT_ID_BYTES },
    };
  });

  // --- build unsigned tx for sighash -----------------------------------------
  const tx = {
    version: 1,
    inputs: [{ prevTxId: hexToBytes(inputTxId), prevIndex: inputIndex, sequence: 0n }],
    outputs,
    lockTime: 0n,
    subnetworkId: new Uint8Array(20),
    gas: 0n,
    payload: new Uint8Array(0),
  };

  const utxoEntries = [{
    amount: BigInt(inputValue),
    scriptPubKeyVersion: 0,
    scriptPubKey: inputSpk,
  }];

  // --- sighash ---------------------------------------------------------------
  const sigHash = calcSchnorrSignatureHash(tx, 0, utxoEntries, SIG_HASH_ALL);

  // --- schnorr sign ----------------------------------------------------------
  const sig = schnorr.sign(sigHash, hexToBytes(privkeyHex));
  const sigWithType = new Uint8Array(65);
  sigWithType.set(sig, 0);
  sigWithType[64] = SIG_HASH_ALL;

  // --- build sigscript (ABI + redeem push) -----------------------------------
  const abiStates = newStates.map(st => ({
    owner: hexToBytes(st.owner),
    identifierType: 0,
    amount: BigInt(st.amount),
    isMinter: false,
    forSale: !!st.forSale,
    price: BigInt(st.price || 0),
  }));

  const inputRedeemScript = deriveRedeemScript(
    inputOwner, BigInt(inputAmount), !!inputForSale, BigInt(inputPrice || 0)
  );
  const sigscript = buildSigScript(
    abiStates, [sigWithType], new Uint8Array([0x00]), inputRedeemScript, true
  );

  // --- format JSON for submitTransaction -------------------------------------
  const txJson = {
    transaction: {
      version: 1,
      inputs: [{
        previousOutpoint: { transactionId: inputTxId, index: inputIndex },
        signatureScript: bytesToHex(sigscript),
        sequence: 0,
        sigOpCount: 0,
        computeBudget: COMPUTE_BUDGET,
      }],
      outputs: outputs.map((o, i) => ({
        value: Number(o.value),
        scriptPublicKey: spkJsonHex(0, o.scriptPubKey),
        covenant: { authorizingInput: 0, covenantId: COVENANT_ID_HEX },
        verboseData: null,
      })),
      lockTime: 0,
      subnetworkId: SUBNETWORK_NATIVE_HEX,
      gas: 0,
      payload: '',
      mass: 0,
      verboseData: null,
    },
    allowOrphan: false,
  };

  return {
    txJson,
    sigHashHex: bytesToHex(sigHash),
    sigHex: bytesToHex(sigWithType),
    sigscriptLen: sigscript.length,
    outputValues: outputValues.map(v => Number(v)),
  };
}

// --- convenience: build a send tx --------------------------------------------
// sendParams = {
//   inputTxId, inputIndex, inputValue, inputOwner, inputAmount,
//   recipientPubkey (hex), sendAmount (BigInt), privkeyHex
// }
function buildSendTx(p) {
  const whole = (BigInt(p.sendAmount) === BigInt(p.inputAmount));
  const states = whole
    ? [{ owner: p.recipientPubkey, amount: BigInt(p.sendAmount), forSale: false, price: 0n }]
    : [
        { owner: p.recipientPubkey, amount: BigInt(p.sendAmount), forSale: false, price: 0n },
        { owner: p.inputOwner, amount: BigInt(p.inputAmount) - BigInt(p.sendAmount), forSale: false, price: 0n },
      ];
  return buildOwnerSignedTx({
    ...p, inputForSale: false, inputPrice: 0n, newStates: states,
  });
}

// --- convenience: build a split tx -------------------------------------------
function buildSplitTx(p) {
  // p = { ..., splitAmount (BigInt, first piece) }
  const states = [
    { owner: p.inputOwner, amount: BigInt(p.splitAmount), forSale: false, price: 0n },
    { owner: p.inputOwner, amount: BigInt(p.inputAmount) - BigInt(p.splitAmount), forSale: false, price: 0n },
  ];
  return buildOwnerSignedTx({
    ...p, inputForSale: false, inputPrice: 0n, newStates: states,
  });
}

// --- convenience: build a list tx --------------------------------------------
function buildListTx(p) {
  // p = { ..., listPrice (BigInt, sompi) }
  const states = [
    { owner: p.inputOwner, amount: BigInt(p.inputAmount), forSale: true, price: BigInt(p.listPrice) },
  ];
  return buildOwnerSignedTx({
    ...p, inputForSale: false, inputPrice: 0n, newStates: states,
  });
}

// --- self-test: cross-check sighash against CLI dry-run ----------------------
//
// The CLI split dry-run (buyer2, 5000→1000+4000) produced:
//   sighash: aef9823c78ef4bae0c91fa4e09489f93d9ebb4f61fbf103258bf9147d70df13c
//
// We rebuild the exact same tx and verify the sighash matches. This proves:
//   - P2SH derivation for input + outputs → correct scriptPubKeys
//   - sighash computation with covenant bindings → byte-identical to Rust
//   - proportional value split → same output values
//
// NOTE: we use a DUMMY private key for the self-test. The sighash is computed
// from the UNSIGNED tx (empty sigscript), so the key doesn't affect it.
//
function selfTest() {
  const buyer2 = '5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161';
  // Dummy key — sighash is computed before signing, so any key works for the cross-check
  const dummyPriv = '0000000000000000000000000000000000000000000000000000000000000001';

  let result;
  try {
    result = buildSplitTx({
      inputTxId: '28a90bc9a5ae922837e05f10952be500be289709cca8cf55461d6fd6ed5fec35',
      inputIndex: 0,
      inputValue: 163036283150n,
      inputOwner: buyer2,
      inputAmount: 5000n,
      splitAmount: 1000n,
      privkeyHex: dummyPriv,
    });
  } catch (e) {
    console.log('[tx selfTest] BUILD FAILED:', e.message);
    return false;
  }

  const expectedSigHash = 'aef9823c78ef4bae0c91fa4e09489f93d9ebb4f61fbf103258bf9147d70df13c';
  const ok = result.sigHashHex === expectedSigHash;

  console.log('[tx selfTest] sighash-crosscheck',
    ok ? 'PASS' : 'FAIL',
    ok ? '' : '\n  got: ' + result.sigHashHex + '\n  exp: ' + expectedSigHash);

  if (ok) {
    console.log('[tx selfTest] output values:', result.outputValues);
    console.log('[tx selfTest] sigscript length:', result.sigscriptLen, 'bytes');
  }

  return ok;
}

/* ============================================================================
   CLAIM — 2-input, payment-authorized (ported from bitphoque_sale_claim_v3.rs)
   ----------------------------------------------------------------------------
   A buyer claims a for-sale covenant parcel by PAYING the seller, with NO
   seller signature. This is the unsigned-spend path.

   Tx shape (output ORDER is contract-enforced — witness byte points at output 1):
     input  0: for-sale covenant UTXO     UNSIGNED   (payment-authorized)
     input  1: buyer plain P2PK funding   buyer-SIGNED at index 1
     output 0: token -> BUYER             covenant-bound, authorizing_input 0,
                                          full parcel value, forSale=false
     output 1: payment -> SELLER P2PK     >= price (witness [1] points here)
     output 2: change  -> BUYER  P2PK

   PARTY-AGNOSTIC: the seller is identified by PUBKEY (a param), never a key.
   The claim never needs the seller's private key — that is the whole point.

   KEY DIFFERENCES from buildOwnerSignedTx (why this is a separate fn, not an
   edit to a proven path):
     - 2 inputs, asymmetric: covenant input UNSIGNED, plain input signed.
     - sighash computed over input 1 of a 2-INPUT tx (both UTXO entries present).
     - covenant output carries FULL parcel value; the buyer's separate funding
       UTXO pays seller + change. Not the value-proportional split model.
     - input 0 sigscript uses a DUMMY 65-byte-zero sig + witness=[paymentIdx].
   ============================================================================ */

// claim constants (status doc consensus rules #6, #9)
const CLAIM_FEE = 900_000n;          // v3 2-input claim floor 857k -> 900k headroom
const CLAIM_PAYMENT = 100_000_000n;  // 1 TKAS — KIP-9-safe; tiny payment spikes storage mass
const PAYMENT_OUTPUT_INDEX = 1;      // witness byte -> output 1 is the seller payment

// merge constants (status doc consensus rule #10)
const MERGE_FEE = 1_300_000n;         // v3 merge 2→1 floor ~1,244,500 → 1.3M margin
const MERGE_COMPUTE_BUDGET = 30;      // 300k script units (two covenant reveals) → ceil(300000/10000)

// Plain P2PK scriptPubKey: OP_DATA32 <xonly-pubkey> OP_CHECKSIG = 20<pubkey>ac
// (Kaspa schnorr P2PK. NOT a covenant script — claim's funding input + payment
//  + change outputs are all plain P2PK, which the covenant builders never emit.)
function makeP2pk(pubkeyHex) {
  return hexToBytes('20' + pubkeyHex + 'ac');
}

// buildClaimTx(params)
// params = {
//   covTxId:        string  — for-sale covenant UTXO txid (64-hex)
//   covIndex:       number  — its output index
//   covValue:       BigInt  — its sompi value (carried whole to the token output)
//   sellerPubkey:   string  — 64-hex x-only; identifies the for-sale parcel owner
//   buyerPubkey:    string  — 64-hex x-only; the claimer (new token owner)
//   amount:         BigInt  — phoques in the parcel (unchanged by claim)
//   price:          BigInt  — the listing price (sompi); payment must be >= this
//   fundTxId:       string  — buyer's plain P2PK funding UTXO txid (64-hex)
//   fundIndex:      number  — its output index
//   fundValue:      BigInt  — its sompi value (must exceed payment + fee)
//   buyerPrivkeyHex:string  — buyer's 32-byte private key (64-hex); signs input 1
// }
// Returns { txJson, sigHashHex, sigHex, sigscriptLen, paymentValue, changeValue }
function buildClaimTx(params) {
  const {
    covTxId, covIndex, covValue,
    sellerPubkey, buyerPubkey, amount, price,
    fundTxId, fundIndex, fundValue, buyerPrivkeyHex,
  } = params;

  const payment = CLAIM_PAYMENT;
  if (payment < BigInt(price))
    throw new Error('payment ' + payment + ' < price ' + price + ' (contract requires payment >= price)');
  if (BigInt(fundValue) <= payment + CLAIM_FEE)
    throw new Error('funding ' + fundValue + ' <= payment+fee ' + (payment + CLAIM_FEE));

  // --- scriptPubKeys ---------------------------------------------------------
  // input 0: the for-sale covenant P2SH (seller-owned, forSale=true, price)
  const covSpkHash = deriveScriptHashHex(sellerPubkey, BigInt(amount), true, BigInt(price));
  const covSpk = makeSpk(covSpkHash);
  // input 1: buyer's plain P2PK funding UTXO
  const buyerP2pk = makeP2pk(buyerPubkey);
  // output 0: token -> buyer, covenant P2SH (buyer-owned, forSale=false, price 0)
  const tokenSpkHash = deriveScriptHashHex(buyerPubkey, BigInt(amount), false, 0n);
  const tokenSpk = makeSpk(tokenSpkHash);
  // output 1: payment -> seller P2PK ; output 2: change -> buyer P2PK
  const sellerP2pk = makeP2pk(sellerPubkey);

  const changeValue = BigInt(fundValue) - payment - CLAIM_FEE;

  // --- outputs (ORDER MATTERS: token, payment, change) -----------------------
  const outputs = [
    { // 0: token -> buyer (covenant)
      value: BigInt(covValue),
      scriptPubKeyVersion: 0,
      scriptPubKey: tokenSpk,
      covenant: { authorizingInput: 0, covenantId: COVENANT_ID_BYTES },
    },
    { // 1: payment -> seller (plain)
      value: payment,
      scriptPubKeyVersion: 0,
      scriptPubKey: sellerP2pk,
      covenant: null,
    },
    { // 2: change -> buyer (plain)
      value: changeValue,
      scriptPubKeyVersion: 0,
      scriptPubKey: buyerP2pk,
      covenant: null,
    },
  ];

  // --- unsigned 2-input tx for the buyer's sighash (signed at index 1) -------
  const tx = {
    version: 1,
    inputs: [
      { prevTxId: hexToBytes(covTxId),  prevIndex: covIndex,  sequence: 0n },
      { prevTxId: hexToBytes(fundTxId), prevIndex: fundIndex, sequence: 0n },
    ],
    outputs,
    lockTime: 0n,
    subnetworkId: new Uint8Array(20),
    gas: 0n,
    payload: new Uint8Array(0),
  };

  // UTXO entries, one per input, SAME ORDER. (No covenant field needed here —
  // the sighash hashes each input's scriptPubKey, not its covenant binding;
  // the covenant_id lives on the token OUTPUT, already in `outputs` above.)
  const utxoEntries = [
    { amount: BigInt(covValue),  scriptPubKeyVersion: 0, scriptPubKey: covSpk },
    { amount: BigInt(fundValue), scriptPubKeyVersion: 0, scriptPubKey: buyerP2pk },
  ];

  // --- sighash for INPUT 1 (the buyer's plain funding input) -----------------
  const sigHash = calcSchnorrSignatureHash(tx, 1, utxoEntries, SIG_HASH_ALL);
  const sig = schnorr.sign(sigHash, hexToBytes(buyerPrivkeyHex));
  const sigWithType = new Uint8Array(65);
  sigWithType.set(sig, 0);
  sigWithType[64] = SIG_HASH_ALL;
  const in1Sigscript = buildPlainSig(sigWithType); // just a single sig push

  // --- INPUT 0 covenant leader sigscript -------------------------------------
  // new-state: buyer-owned, forSale=false, price 0 ; dummy 65-zero sig ;
  // witness = single byte [paymentOutputIndex] ; leader ; redeem push.
  const newState = {
    owner: hexToBytes(buyerPubkey),
    identifierType: 0,
    amount: BigInt(amount),
    isMinter: false,
    forSale: false,
    price: 0n,
  };
  const dummySig = new Uint8Array(65); // all zero — for-sale branch is payment-authorized
  const witnesses = new Uint8Array([PAYMENT_OUTPUT_INDEX]);
  const covRedeem = deriveRedeemScript(sellerPubkey, BigInt(amount), true, BigInt(price));
  const in0Sigscript = buildSigScript([newState], [dummySig], witnesses, covRedeem, true);

  // --- JSON for submitTransaction (2 inputs) ---------------------------------
  const txJson = {
    transaction: {
      version: 1,
      inputs: [
        { // 0: covenant, unsigned-spend (sigscript carries ABI + redeem, no real sig)
          previousOutpoint: { transactionId: covTxId, index: covIndex },
          signatureScript: bytesToHex(in0Sigscript),
          sequence: 0,
          sigOpCount: 0,
          computeBudget: COMPUTE_BUDGET,
        },
        { // 1: buyer plain funding, signed
          previousOutpoint: { transactionId: fundTxId, index: fundIndex },
          signatureScript: bytesToHex(in1Sigscript),
          sequence: 0,
          sigOpCount: 0,
          computeBudget: 11,
        },
      ],
      outputs: outputs.map((o) => ({
        value: Number(o.value),
        scriptPublicKey: spkJsonHex(0, o.scriptPubKey),
        covenant: o.covenant ? { authorizingInput: 0, covenantId: COVENANT_ID_HEX } : null,
        verboseData: null,
      })),
      lockTime: 0,
      subnetworkId: SUBNETWORK_NATIVE_HEX,
      gas: 0,
      payload: '',
      mass: 0,
      verboseData: null,
    },
    allowOrphan: false,
  };

  return {
    txJson,
    sigHashHex: bytesToHex(sigHash),
    sigHex: bytesToHex(sigWithType),
    sigscriptLen: in0Sigscript.length,
    paymentValue: Number(payment),
    changeValue: Number(changeValue),
  };
}

// A plain P2PK sigscript is just the signature pushed (OP_DATA-len framing).
// (buildSigScript is covenant-ABI only; the buyer's funding input is plain.)
function buildPlainSig(sig65) {
  const n = sig65.length;        // 65 -> single-byte length prefix
  const out = new Uint8Array(1 + n);
  out[0] = n;
  out.set(sig65, 1);
  return out;
}

// --- claim self-test: cross-check input-1 sighash against CLI harness ---------
//
// Reference captured from a `bitphoque_sale_claim_v3` dry-run (2026-06-26):
//   seller buyer1 (8c02c20a…), buyer3 (267496ce…) claims the 3000 parcel,
//   price 500000, payment 100000000, fee 900000.
//   cov UTXO   8dff6df4…:0  value 97821666447
//   fund UTXO  40cbf865…:5  value 99898500000
//   harness printed:  buyer sighash de52d220…  ;  VM verify PASS.
//
// Rebuild the IDENTICAL tx in JS and assert the input-1 sighash matches. This
// proves the 2-input claim path: P2SH (input/token) + P2PK (fund/pay/change)
// scriptPubKeys, output ORDER, covenant binding on output 0, and the sighash
// computed over input 1 of a 2-input tx — all byte-identical to the Rust engine.
// (Dummy key: the sighash is over the UNSIGNED tx, so signing key is irrelevant.)
//
function claimSelfTest() {
  const REF = 'de52d220051a7bc78426de553835b0ccd9aedf70a20341f721f423be70c1bae1';
  const dummyPriv = '0000000000000000000000000000000000000000000000000000000000000001';

  let r;
  try {
    r = buildClaimTx({
      covTxId: '8dff6df4fa58e6efa72998d4914a7b387c8b0bacddccb1a1e61ebfc4c96f7a48',
      covIndex: 0,
      covValue: 97821666447n,
      sellerPubkey: '8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936',
      buyerPubkey:  '267496ce01f2a35653491c61b56b23e6b7dbe777a02b146286ac3635ff36ab61',
      amount: 3000n,
      price: 500000n,
      fundTxId: '40cbf8659db6e0d17465c0916da4581557483e8a5e3d204e91e12d085b79d80d',
      fundIndex: 5,
      fundValue: 99898500000n,
      buyerPrivkeyHex: dummyPriv,
    });
  } catch (e) {
    console.log('[claim selfTest] BUILD FAILED:', e.message);
    return false;
  }

  const ok = r.sigHashHex === REF;
  console.log('[claim selfTest] sighash-crosscheck', ok ? 'PASS' : 'FAIL',
    ok ? '' : '\n  got: ' + r.sigHashHex + '\n  exp: ' + REF);
  if (ok) {
    console.log('[claim selfTest] payment', r.paymentValue, r.paymentValue === 100000000 ? 'OK' : 'BAD');
    console.log('[claim selfTest] change ', r.changeValue, r.changeValue === 99797600000 ? 'OK' : 'BAD');
    console.log('[claim selfTest] in0 sigscript len', r.sigscriptLen, 'bytes');
  }
  return ok;
}

/* ============================================================================
   MERGE — 2-input leader/delegate, owner-signed (ported from cmd_merge in CLI)
   ----------------------------------------------------------------------------
   Merges two covenant parcels (same owner, both not-for-sale) into one.
   This is the first MULTI-COVENANT-INPUT tx in the wallet.

   Tx shape:
     input  0: covenant UTXO (parcel A)   LEADER   owner-signed at sighash index 0
     input  1: covenant UTXO (parcel B)   DELEGATE selector + redeem only
     output 0: merged covenant UTXO       amount = A + B, same owner, forSale=false

   KEY DIFFERENCES from single-input txs:
     - 2 covenant inputs, each with its own redeem script (different amounts →
       different P2SH → different redeem scripts).
     - Leader ABI carries TWO sigs (one per covenant input), both computed
       against sighash index 0. Same key + same sighash = identical bytes,
       but the ABI expects one entry per covenant input.
     - Witnesses = [0x00, 0x01] — dormant filler on the owner-signed branch;
       the contract doesn't check them, but the ABI expects the blob.
     - Delegate sigscript is JUST selector (OP_1) + redeem push — no ABI args.
     - computeBudget = 30 (300k script units for two covenant reveals),
       not 24 like single-covenant-input txs.
     - No plain P2PK inputs or outputs. Fee comes from the covenant UTXOs' KAS.
   ============================================================================ */

// buildMergeTx(params)
// params = {
//   in0TxId:      string  — parcel 0 txid (64-hex)
//   in0Index:     number  — parcel 0 output index
//   in0Value:     BigInt  — parcel 0 sompi value
//   in0Amount:    BigInt  — parcel 0 phoques
//   in1TxId:      string  — parcel 1 txid (64-hex)
//   in1Index:     number  — parcel 1 output index
//   in1Value:     BigInt  — parcel 1 sompi value
//   in1Amount:    BigInt  — parcel 1 phoques
//   ownerPubkey:  string  — 64-hex x-only (same owner, both parcels)
//   privkeyHex:   string  — 64-hex private key (signs both inputs)
// }
// Returns { txJson, sigHashHex, sigHex, in0SigscriptLen, in1SigscriptLen, outputValue }
function mergeSelfTest() {
  const REF = 'f8e741824120ad22ca334be7be3cf892412bdc449fd37b7e240b69afa4dc6e6e';
  const dummyPriv = '0000000000000000000000000000000000000000000000000000000000000001';

  let r;
  try {
    r = buildMergeTx({
      in0TxId: 'b4ab26d26a6248a8ac465878f2bc91d81ea512647b0206e2afbc4a353d119eb4',
      in0Index: 1,
      in0Value: 130428466520n,
      in0Amount: 4000n,
      in1TxId: 'b52e19c6c629655565f4fa98b016fdc36199841db515e55007cb6643f18435ba',
      in1Index: 0,
      in1Value: 531528968714n,
      in1Amount: 16301n,
      ownerPubkey: '5d338aa54bcf4d955941ff8bcdd069b2f3aa1deb40234dc73ce5f4f581adf161',
      privkeyHex: dummyPriv,
    });
  } catch (e) {
    console.log('[merge selfTest] BUILD FAILED:', e.message);
    return false;
  }

  const ok = r.sigHashHex === REF;
  console.log('[merge selfTest] sighash-crosscheck', ok ? 'PASS' : 'FAIL',
    ok ? '' : '\n  got: ' + r.sigHashHex + '\n  exp: ' + REF);
  if (ok) {
    console.log('[merge selfTest] outputValue', r.outputValue, r.outputValue === 661956135234 ? 'OK' : 'BAD');
    console.log('[merge selfTest] in0 sigscript len', r.in0SigscriptLen, 'bytes');
    console.log('[merge selfTest] in1 sigscript len', r.in1SigscriptLen, 'bytes');
  }
  return ok;
}
function buildMergeTx(params) {
  const {
    in0TxId, in0Index, in0Value, in0Amount,
    in1TxId, in1Index, in1Value, in1Amount,
    ownerPubkey, privkeyHex,
  } = params;

  const outAmount = BigInt(in0Amount) + BigInt(in1Amount);
  const totalValue = BigInt(in0Value) + BigInt(in1Value);
  if (totalValue <= MERGE_FEE)
    throw new Error('combined value ' + totalValue + ' <= merge fee ' + MERGE_FEE);
  const outValue = totalValue - MERGE_FEE;
  if (outValue < MIN_OUT_VALUE)
    throw new Error('output value ' + outValue + ' below MIN_OUT_VALUE');

  // --- input scriptPubKeys (for sighash) — both not-for-sale, price 0 --------
  const in0SpkHash = deriveScriptHashHex(ownerPubkey, BigInt(in0Amount), false, 0n);
  const in0Spk = makeSpk(in0SpkHash);
  const in1SpkHash = deriveScriptHashHex(ownerPubkey, BigInt(in1Amount), false, 0n);
  const in1Spk = makeSpk(in1SpkHash);

  // --- output scriptPubKey (merged parcel) -----------------------------------
  const outSpkHash = deriveScriptHashHex(ownerPubkey, outAmount, false, 0n);
  const outSpk = makeSpk(outSpkHash);

  // --- single covenant output ------------------------------------------------
  const outputs = [{
    value: outValue,
    scriptPubKeyVersion: 0,
    scriptPubKey: outSpk,
    covenant: { authorizingInput: 0, covenantId: COVENANT_ID_BYTES },
  }];

  // --- unsigned 2-input tx for sighash ---------------------------------------
  const tx = {
    version: 1,
    inputs: [
      { prevTxId: hexToBytes(in0TxId), prevIndex: in0Index, sequence: 0n },
      { prevTxId: hexToBytes(in1TxId), prevIndex: in1Index, sequence: 0n },
    ],
    outputs,
    lockTime: 0n,
    subnetworkId: new Uint8Array(20),
    gas: 0n,
    payload: new Uint8Array(0),
  };

  const utxoEntries = [
    { amount: BigInt(in0Value), scriptPubKeyVersion: 0, scriptPubKey: in0Spk },
    { amount: BigInt(in1Value), scriptPubKeyVersion: 0, scriptPubKey: in1Spk },
  ];

  // --- sighash at INDEX 0 (leader) — BOTH sigs use this ----------------------
  const sigHash = calcSchnorrSignatureHash(tx, 0, utxoEntries, SIG_HASH_ALL);
  const sig = schnorr.sign(sigHash, hexToBytes(privkeyHex));
  const sigWithType = new Uint8Array(65);
  sigWithType.set(sig, 0);
  sigWithType[64] = SIG_HASH_ALL;

  // --- leader sigscript (input 0): full ABI ----------------------------------
  // One new state: merged parcel (outAmount, same owner, not-for-sale)
  const newState = {
    owner: hexToBytes(ownerPubkey),
    identifierType: 0,
    amount: outAmount,
    isMinter: false,
    forSale: false,
    price: 0n,
  };
  // Two sigs in the array (one per covenant input). Same key + same sighash =
  // identical bytes, but the ABI expects one entry per covenant input.
  const witnesses = new Uint8Array([0x00, 0x01]); // dormant filler, match-proven
  const in0Redeem = deriveRedeemScript(ownerPubkey, BigInt(in0Amount), false, 0n);
  const in0Sigscript = buildSigScript(
    [newState], [sigWithType, sigWithType], witnesses, in0Redeem, true
  );

  // --- delegate sigscript (input 1): selector + redeem only ------------------
  const in1Redeem = deriveRedeemScript(ownerPubkey, BigInt(in1Amount), false, 0n);
  const in1Sigscript = buildSigScript(null, null, null, in1Redeem, false);

  // --- JSON for submitTransaction (2 covenant inputs) ------------------------
  const txJson = {
    transaction: {
      version: 1,
      inputs: [
        { // 0: leader covenant input
          previousOutpoint: { transactionId: in0TxId, index: in0Index },
          signatureScript: bytesToHex(in0Sigscript),
          sequence: 0,
          sigOpCount: 0,
          computeBudget: MERGE_COMPUTE_BUDGET,
        },
        { // 1: delegate covenant input
          previousOutpoint: { transactionId: in1TxId, index: in1Index },
          signatureScript: bytesToHex(in1Sigscript),
          sequence: 0,
          sigOpCount: 0,
          computeBudget: MERGE_COMPUTE_BUDGET,
        },
      ],
      outputs: [{
        value: Number(outValue),
        scriptPublicKey: spkJsonHex(0, outSpk),
        covenant: { authorizingInput: 0, covenantId: COVENANT_ID_HEX },
        verboseData: null,
      }],
      lockTime: 0,
      subnetworkId: SUBNETWORK_NATIVE_HEX,
      gas: 0,
      payload: '',
      mass: 0,
      verboseData: null,
    },
    allowOrphan: false,
  };

  return {
    txJson,
    sigHashHex: bytesToHex(sigHash),
    sigHex: bytesToHex(sigWithType),
    in0SigscriptLen: in0Sigscript.length,
    in1SigscriptLen: in1Sigscript.length,
    outputValue: Number(outValue),
  };
}

// --- attach to wallet namespace if present -----------------------------------
if (typeof window !== 'undefined' && window.KASPA) {
  window.KASPA.buildSendTx = buildSendTx;
  window.KASPA.buildSplitTx = buildSplitTx;
  window.KASPA.buildListTx = buildListTx;
  window.KASPA.buildClaimTx = buildClaimTx;
  window.KASPA.buildMergeTx = buildMergeTx;
  window.KASPA.buildOwnerSignedTx = buildOwnerSignedTx;
  window.KASPA.txSelfTest = selfTest;
  window.KASPA.claimSelfTest = claimSelfTest;
  window.KASPA.mergeSelfTest = mergeSelfTest;
}

export {
  buildOwnerSignedTx, buildSendTx, buildSplitTx, buildListTx, buildClaimTx, buildMergeTx,
  selfTest, claimSelfTest, mergeSelfTest,
  FEE, COMPUTE_BUDGET, COVENANT_ID_HEX, MIN_OUT_VALUE,
  CLAIM_FEE, CLAIM_PAYMENT,
  MERGE_FEE, MERGE_COMPUTE_BUDGET,
};
