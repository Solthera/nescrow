## Variant: Wallet dark

### Design stance
Behave like a self-custody wallet: the number is the hero, the chain is the frame,
and the primary action sits within thumb reach.

### Key choices
- Layout: single column, phone width (430px) inside a desktop frame
- Typography: system sans for labels, monospace with tabular figures for every amount
- Color: near-black ground with one radial glow; a single lime accent, used only for
  "this is the live path" (progress fill, current node, primary button)
- Status: a small mono chip, not a colored pill
- Time: a 4px meter plus a five-node rail — the escrow's clock is visible, not implied
- Interaction: "Revise" opens a bottom sheet

### Trade-offs
- Strong at: feeling like a real crypto product, glanceable balance, thumb-first actions
- Weak at: desktop density — this wants to be a phone, and a wide screen shows one column

### Best for
Demo video on a laptop with a phone-width frame, and for judges who already use wallets.
