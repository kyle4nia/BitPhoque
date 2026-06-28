# BitPhoque Wallet v2 -- UI Design Spec

_The wallet stops being "six panels" and becomes a view of your phoques._

---

## Core principle

The user's mental model: "I have phoques. What's happening with them?"
Everything flows from that. The wallet reads the chain and presents state.
The user never assembles the picture from raw parts.

---

## Home screen

No panel menu. One screen shows everything.

**Header:** Seal + "BitPhoque" + network badge. Two icon buttons: QR (share
identity) and settings (gear).

**Big number:** Total phoques, centered, unmissable. Below it: KAS balance
for fees, shown quietly. If KAS is too low to transact, a plain warning:
"You need KAS to send phoques."

**Parcel cards:** Each card = one group of phoques + its on-chain state.
Cards are color-coded by state:

| State     | Color  | Icon             | Subtitle example         |
|-----------|--------|------------------|--------------------------|
| Held      | Green  | wallet           | "Held"                   |
| For sale  | Blue   | tag              | "For sale - 2 KAS"       |
| Incoming  | Amber  | arrow-down-left  | "Incoming - confirming"  |
| Sending   | Amber  | arrow-up-right   | "Sending - confirming"   |
| Sold      | Green  | check            | "Sold - 2 KAS received"  |

Tap a card: contextual actions expand inline (not a new screen):
- **Held** -> Send, Sell, Split (only shown if relevant)
- **For sale** -> Delist, Change price
- **Incoming** -> shows status, auto-claims when confirmed
- **Sending** -> shows status, no action needed

**Primary actions (bottom):** Two buttons: "Send" and "Receive." These are
the two things you initiate. Everything else is contextual on cards.

**Footer:** Green dot + "Connected to node" (or red + "Disconnected").

---

## Four user stories (not six panels)

### 1. Send phoques

Tap Send (or tap a held card -> Send).

**Screen: "Send phoques"**
- **To:** Paste pubkey or scan QR. The only thing the user types/scans is
  the recipient's identity.
- **Amount:** Number input. Shows "of N held" so the user knows bounds.
- **Info box:** Plain English summary. "This will send 3 and keep 2 in your
  wallet." The word "split" never appears.
- **Summary card:** Sending / Keeping / Network fee.
- **Button:** "Send 3 phoques."

If the amount equals the parcel exactly, no split needed (the wallet knows).
If the amount requires a split, the wallet handles it silently.

**After confirm: "Sent!" screen**
- Shows a QR code + copy button for the descriptor.
- Plain text: "Share this with the recipient so they can claim."
- The word "descriptor" appears only in a details fold or footnote.
- The home screen card updates to "Sending - confirming" (or disappears
  once confirmed, with the kept portion showing as a new "Held" card).

### 2. Receive phoques

Tap Receive.

**Screen: "Receive phoques"**
- Top section: your identity QR code. "Share this with the sender."
- Below: paste/scan field for the code the sender gives back.
- Inline instructions (3 steps, numbered, plain English):
  1. Share your QR code with the sender.
  2. They send phoques and give you a code back.
  3. Scan or paste that code here to claim.

**After scanning/pasting a valid descriptor:**
- Shows: amount, sender short-id, on-chain status, covenant verification.
- Green checkmarks for "confirmed" and "verified."
- Button: "Claim 3 phoques."
- Footer: "The chain is the truth. The wallet verified this parcel exists,
  matches the BitPhoque covenant, and belongs to your key."

**After claim:** Card appears on home screen as "Held."

### 3. Sell phoques

Tap a held card -> Sell.

**Screen: "Sell phoques"**
- Amount is pre-filled (the card's amount). If the user wants to sell a
  partial amount, the wallet offers to split first (framed as "Sell 3 of
  your 5?").
- **Price:** KAS amount input.
- **Summary:** Listing / Price / Fee.
- **Button:** "List for sale."

**After listing:** Card on home screen changes to "For sale - 2 KAS."
The listing is a shareable QR/link. The wallet generates it automatically.

### 4. Buy phoques

Someone shares a listing (QR/link). User scans it.

**Screen: "Buy phoques"**
- Shows: amount, price, seller short-id.
- KAS balance check: if insufficient, plain warning.
- **Button:** "Buy 10 phoques for 2 KAS."

**After purchase:** Card appears on home screen as "Held."

---

## Things that are invisible

- **P2SH addresses.** Derived behind the scenes. Never shown.
- **Outpoints and txids.** Parcels get short readable IDs (first 4 hex
  chars of something unique). Raw data in a "details" fold for power users.
- **The word "parcel."** The user sees "5 phoques." That IS the parcel.
- **Split/merge as operations.** "Send 3 of my 5" triggers split. "Send
  all" from two held cards triggers merge. The user never picks these.
- **Descriptors as text blobs.** They become QR codes or shareable links.
  Copy-paste of base58 is a fallback under a "manual" fold.
- **Sighash, covenant_id, script hash.** Plumbing. Never exposed.

## Things that are visible

- **Total phoques** (big, centered, unmissable)
- **State of each group** (color-coded cards with plain English labels)
- **Your identity** (QR code, one tap, always accessible)
- **KAS balance** (and a warning if too low)
- **Node connection status** (green/red dot)
- **What just happened** (confirmation screens with plain summaries)

---

## The identity layer

Your pubkey is your identity. It's never shown as a 64-char hex string
in normal use. Instead:

- **Short form:** First 4 + last 4 hex chars (a7f3...e2c1). Used in card
  subtitles and sender/recipient labels.
- **QR code:** One-tap share from the top-right button. The recipient scans
  it; the wallet extracts the pubkey.
- **Full hex:** Available under a "details" fold. Power-user territory.

---

## The naming layer (display labels)

Every object carries its own explanation. Not `pubkey.status.amount` as a
protocol string, but as a display principle:

- Your holdings: "5 held - 10 listed - 3 incoming"
- A listing shared with you: "10 phoques - 2 KAS - from [short-id]"
- A send in progress: "3 phoques to [short-id] - confirming"

The label IS the state. No lookup required.

---

## Design language

Inherits the existing ink-on-paper aesthetic (cream/ink palette, Inter +
Fraunces) but shifts from document layout to app layout:

- Cards with rounded corners, color-coded left icons
- State colors: green (held/success), blue (listed/active), amber
  (pending/incoming)
- Minimal borders, generous whitespace
- One primary action button per screen (dark fill)
- Footnotes and technical details in muted text, never prominent

---

## Migration from v1

The underlying code stays: sighash computation, tx building, wRPC calls,
parcel tracking, encrypted persistence. What changes:

1. **Home screen:** Panel buttons replaced by parcel cards + derived state.
2. **Panel rendering:** Instead of one openPanel() function dispatching to
   a form template, each "story" (send/receive/sell/buy) gets its own
   screen with pre-filled context from the tapped card.
3. **Split/merge:** Wrapped into send/sell flows. Not exposed as panels.
4. **Receive:** Descriptor paste/scan replaces the old receive panel.
   Pubkey display moves to the identity QR button.
5. **Address display:** Removed from receive panel. KAS address for fees
   could be shown in a settings/details screen.

The refactor is UI-only. The crypto layer is proven and doesn't change.

---

## Open questions

- **Auto-claim:** Should incoming parcels auto-claim when confirmed, or
  require a manual "Claim" tap? Auto-claim is smoother but less explicit.
- **Descriptor relay:** Long-term, a simple relay service (pastebin-like,
  keyed to pubkey) could eliminate the QR exchange entirely. "3 phoques
  arrived" as a notification. Adds infrastructure -- park for post-testnet.
- **Multiple parcels of same state:** If you have three separate "held"
  groups (5, 3, 2), show three cards or one card saying "10 held (3
  parcels)"? Three cards is more explicit; one card is cleaner.
- **KAS acquisition:** How does a new user get KAS for fees? Link to a
  faucet (testnet) or exchange (mainnet)? Show a prompt when balance is 0?
