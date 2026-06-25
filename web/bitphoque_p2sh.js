/* ============================================================================
   BitPhoque — in-browser P2SH derivation (Route B1: template splice)
   ----------------------------------------------------------------------------
   Replaces the "browser has no silverscript compiler" seam. The v3 covenant
   compiles to a FIXED 2793-byte redeem script whose only per-parcel operands
   (owner / amount / forSale / price) sit at FIXED byte offsets with FIXED
   width — proven by compiling 4 parameter sets and diffing (all 2793 bytes,
   single-localized diffs, no CScriptNum width drift). So the browser builds
   the redeem script by splicing operands into a captured template, then:
       scriptHash = blake2b-256(redeemScript)          (plain, no key — verified vs source)
       scriptPubKey = OP_BLAKE2B OP_DATA32 <hash> OP_EQUAL   = aa20<hash>87
       address = kasEncode(network, 8, scriptHash)     (version 8 = ScriptHash)

   PROVEN (selfTest): parcel A (owner buyer1, 3000, forSale=true, 500000)
       -> hash  f8a010c4...cdaee   (== live on-chain scriptPubKey aa20..87)
       -> addr  kaspatest:pru2qyxyfwcups795dw94tm8k4eaxrvq4ghcms4m6j56hhqq9ndwuw7ajfwk8

   MAINNET-GRADE NOTE (self-erecting lens): this template is a build artifact
   pinned to bitphoque_sale_v3.sil + this silverscript compiler version. If the
   contract source or compiler changes, the offsets can move and the template
   MUST be re-captured (run bitphoque_script_diff, re-verify selfTest). For a
   single fixed covenant this is an acceptable, documented constraint — but it
   is a constraint, not a free lunch. A fuller version would compile in-WASM.
   ============================================================================ */
import { blake2b } from './vendor/noble-hashes/blake2.js';

// --- the captured v3 redeem-script template (parcel A: owner buyer1, amt 3000, fs true, price 500000) ---
// Operand slots are overwritten by deriveRedeemScript(); the rest is invariant boilerplate.
const TEMPLATE_HEX =
  "6b208c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936010008b80b000000000000010001010820a10700000000006c76009c6375b9cf7600d1b99c6976d07653a16978d2000000000000005879009453a1697659799f63765a7978d15b795279d1c902e90a9452935c795379d1c902e90a945293012093bc5b795279d15c795379d1c902e90a940123935d795479d1c902e90a940123935193bc5c795379d15d795479d1c902e90a940125935e795579d1c902e90a940125935893bc5d795479d15e795579d1c902e90a94012e935f795679d1c902e90a94012e935193bc5e795579d15f795679d1c902e90a9401309360795779d1c902e90a940130935193bc5f795679d160795779d1c902e90a940132930111795879d1c902e90a940132935893bc5d7956797e5e7a755d7956797e5e7a755d79567958cd7e5e7a755d79567951cd7e5e7a755d79567951cd7e5e7a755d79567958cd7e5e7a755c7951935e7a75577a75577a75577a75577a75577a75577a75577a75687659799f63765a7978d15b795279d1c902e90a9452935c795379d1c902e90a945293012093bc5b795279d15c795379d1c902e90a940123935d795479d1c902e90a940123935193bc5c795379d15d795479d1c902e90a940125935e795579d1c902e90a940125935893bc5d795479d15e795579d1c902e90a94012e935f795679d1c902e90a94012e935193bc5e795579d15f795679d1c902e90a9401309360795779d1c902e90a940130935193bc5f795679d160795779d1c902e90a940132930111795879d1c902e90a940132935893bc5d7956797e5e7a755d7956797e5e7a755d79567958cd7e5e7a755d79567951cd7e5e7a755d79567951cd7e5e7a755d79567958cd7e5e7a755c7951935e7a75577a75577a75577a75577a75577a75577a75577a75687659799f63765a7978d15b795279d1c902e90a9452935c795379d1c902e90a945293012093bc5b795279d15c795379d1c902e90a940123935d795479d1c902e90a940123935193bc5c795379d15d795479d1c902e90a940125935e795579d1c902e90a940125935893bc5d795479d15e795579d1c902e90a94012e935f795679d1c902e90a94012e935193bc5e795579d15f795679d1c902e90a9401309360795779d1c902e90a940130935193bc5f795679d160795779d1c902e90a940132930111795879d1c902e90a940132935893bc5d7956797e5e7a755d7956797e5e7a755d79567958cd7e5e7a755d79567951cd7e5e7a755d79567951cd7e5e7a755d79567958cd7e5e7a755c7951935e7a75577a75577a75577a75577a75577a75577a75577a7568567956795679567956795679011d79011d79011d79011d79011d79011d795b7958795879012079012079518f005679827c75012096009453a169765779827c750120969f637656797851957651937f9163547978014195760141937f58795279012095760120937fac696753797851957651937f765479a06976547a7558795379012095760120937f030000207c7e01ac7e5579547951957651937fc37887695579547951957651937fc25879557958957658937fa26975777b7b687651937b757768765779827c750120969f637656797851957651937f9163547978014195760141937f58795279012095760120937fac696753797851957651937f765479a06976547a7558795379012095760120937f030000207c7e01ac7e5579547951957651937fc37887695579547951957651937fc25879557958957658937fa26975777b7b687651937b757768765779827c750120969f637656797851957651937f9163547978014195760141937f58795279012095760120937fac696753797851957651937f765479a06976547a7558795379012095760120937f030000207c7e01ac7e5579547951957651937fc37887695579547951957651937fc25879557958957658937fa26975777b7b687651937b7577680112790111795e795d79012379916300005579827c75012096009453a169765679827c750120969f637652795679527958957658937f93537a75785193537a757b7568765679827c750120969f637652795679527958957658937f93537a75785193537a757b7568765679827c750120969f637652795679527958957658937f93537a75785193537a757b756800005579827c75012096009453a169765679827c750120969f637652795679527958957658937f93537a75785193537a757b7568765679827c750120969f637652795679527958957658937f93537a75785193537a757b7568765679827c750120969f637652795679527958957658937f93537a75785193537a757b7568537952799c69757575756860795e790125799163005279827c75012096009453a169765379827c750120969f637652797851957651937f91697651937b757768765379827c750120969f637652797851957651937f91697651937b757768765379827c750120969f637652797851957651937f91697651937b757768756801187901157901147901117900005579827c75012096009453a169765679827c750120969f637655797851957651937f6351537a757b7b687651937b757768765679827c750120969f637655797851957651937f6351537a757b7b687651937b757768765679827c750120969f637655797851957651937f6351537a757b7b687651937b7577687863005479827c75012096009453a169765579827c750120969f637654797851957651937f91697651937b757768765579827c750120969f637654797851957651937f91697651937b757768765579827c750120969f637654797851957651937f91697651937b757768756801267953a169012679013779827c750120969c6900012779009453a169760128799f6376012a7978d30139795279012095760120937f01207c7e013979537951957651937f01017c7e013979547958957658937f58cd01087c7e013979557951957651937f51cd01017c7e013979567951957651937f51cd01017c7e013979577958957658937f58cd01087c7e7e7e7e7e7eb976c902e90a94765193bc7c7eb976c97602e90a94013a937cbc7eaa02000001aa7e01207e7c7e01877e78c38769785193537a75777768760128799f6376012a7978d30139795279012095760120937f01207c7e013979537951957651937f01017c7e013979547958957658937f58cd01087c7e013979557951957651937f51cd01017c7e013979567951957651937f51cd01017c7e013979577958957658937f58cd01087c7e7e7e7e7e7eb976c902e90a94765193bc7c7eb976c97602e90a94013a937cbc7eaa02000001aa7e01207e7c7e01877e78c38769785193537a75777768760128799f6376012a7978d30139795279012095760120937f01207c7e013979537951957651937f01017c7e013979547958957658937f58cd01087c7e013979557951957651937f51cd01017c7e013979567951957651937f51cd01017c7e013979577958957658937f58cd01087c7e7e7e7e7e7eb976c902e90a94765193bc7c7eb976c97602e90a94013a937cbc7eaa02000001aa7e01207e7c7e01877e78c38769785193537a75777768007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a75007a757575757575757575757575757575516776519c6375b9cf00d1b99e6975757575757551677500696868";

// fixed slot offsets (bytes) — proven via bitphoque_script_diff
const OFF_OWNER = 2;     // 32 bytes
const OFF_AMOUNT = 37;   // 8 bytes u64-LE
const OFF_FORSALE = 48;  // 1 byte (00/01)
const OFF_PRICE = 50;    // 8 bytes u64-LE

function hexToBytes(h){ const a=new Uint8Array(h.length/2); for(let i=0;i<a.length;i++) a[i]=parseInt(h.substr(i*2,2),16); return a; }
function bytesToHex(b){ let s=''; for(const x of b) s+=x.toString(16).padStart(2,'0'); return s; }
function u64le(nBig){ const o=new Uint8Array(8); let v=BigInt(nBig); for(let i=0;i<8;i++){ o[i]=Number(v & 0xffn); v>>=8n; } return o; }

const TEMPLATE = hexToBytes(TEMPLATE_HEX);

// Build the redeem script for given parcel params by splicing into the template.
// owner: 32-byte x-only pubkey (hex string or Uint8Array); amount/price: BigInt|number (phoque units / sompi-as-encoded);
// forSale: bool. Returns Uint8Array (2793 bytes).
function deriveRedeemScript(owner, amount, forSale, price){
  const s = TEMPLATE.slice();                 // copy
  const ownerB = (typeof owner === 'string') ? hexToBytes(owner) : owner;
  if (ownerB.length !== 32) throw new Error('owner must be 32-byte x-only pubkey');
  s.set(ownerB, OFF_OWNER);
  s.set(u64le(amount), OFF_AMOUNT);
  s[OFF_FORSALE] = forSale ? 0x01 : 0x00;
  s.set(u64le(price), OFF_PRICE);
  return s;
}

// scriptHash = blake2b-256(redeemScript), plain (no key) — matches Params::new().hash_length(32)
function scriptHash(redeemScript){ return blake2b(redeemScript, { dkLen: 32 }); }

// Derive the P2SH bech32 address for a parcel. Uses the wallet's proven kasEncode (version 8 = ScriptHash).
// kasEncode is expected on window.KASPA.encode (already vendored & verified vs P2PK vector + KasWare).
function deriveP2SHAddress(owner, amount, forSale, price, network){
  const net = network || (window.KASPA && window.KASPA.network) || 'kaspatest';
  const rs = deriveRedeemScript(owner, amount, forSale, price);
  const h = scriptHash(rs);
  return window.KASPA.encode(net, 8, Array.from(h));   // version 8 = ScriptHash
}

// also expose the raw script-hash hex (== the 32 bytes inside scriptPubKey aa20<hash>87)
function deriveScriptHashHex(owner, amount, forSale, price){
  return bytesToHex(scriptHash(deriveRedeemScript(owner, amount, forSale, price)));
}

// ---- correctness gates: must reproduce live on-chain values for parcel A ----
function selfTest(){
  const ownerA = '8c02c20a67901128dab9377bcb3d9739296e82ac2d9e7628635daf63e63d4936';
  const expectHash = 'f8a010c44bb1c0c3c5a35c5aaf67b573d30d80aa2f8dc2bbd4a9abdc002cdaee';
  const expectAddr = 'kaspatest:pru2qyxyfwcups795dw94tm8k4eaxrvq4ghcms4m6j56hhqq9ndwuw7ajfwk8';
  const gotHash = deriveScriptHashHex(ownerA, 3000n, true, 500000n);
  const gotAddr = deriveP2SHAddress(ownerA, 3000n, true, 500000n, 'kaspatest');
  const okHash = gotHash === expectHash;
  const okAddr = gotAddr === expectAddr;
  // round-trip integrity: re-splice template's own A params must equal the template
  const reA = bytesToHex(deriveRedeemScript(ownerA, 3000n, true, 500000n));
  const okTpl = reA === TEMPLATE_HEX;
  console.log('[p2sh selfTest] hash', okHash ? 'PASS' : 'FAIL', okHash ? '' : `got ${gotHash}`);
  console.log('[p2sh selfTest] addr', okAddr ? 'PASS' : 'FAIL', okAddr ? '' : `got ${gotAddr}`);
  console.log('[p2sh selfTest] tmpl', okTpl ? 'PASS' : 'FAIL');
  return okHash && okAddr && okTpl;
}

// attach to the wallet's KASPA namespace
if (window.KASPA){
  window.KASPA.deriveRedeemScript = deriveRedeemScript;
  window.KASPA.deriveScriptHashHex = deriveScriptHashHex;
  window.KASPA.deriveP2SHAddress = deriveP2SHAddress;
  window.KASPA.p2shSelfTest = selfTest;
}

export { deriveRedeemScript, deriveScriptHashHex, deriveP2SHAddress, selfTest };
