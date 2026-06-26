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

// --- attach to wallet namespace if present -----------------------------------
if (typeof window !== 'undefined' && window.KASPA) {
  window.KASPA.buildSendTx = buildSendTx;
  window.KASPA.buildSplitTx = buildSplitTx;
  window.KASPA.buildListTx = buildListTx;
  window.KASPA.buildOwnerSignedTx = buildOwnerSignedTx;
  window.KASPA.txSelfTest = selfTest;
}

export {
  buildOwnerSignedTx, buildSendTx, buildSplitTx, buildListTx, selfTest,
  FEE, COMPUTE_BUDGET, COVENANT_ID_HEX, MIN_OUT_VALUE,
};
