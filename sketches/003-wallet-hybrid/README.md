## Variant: Wallet hybrid

### Design stance
The wallet mood of 001 (dark ground, one live accent) on the desktop structure of 002
(money left, terms right), so the screen reads at 1080p without zooming.

### Key choices
- Layout: 1120px two-column grid, collapse to one column under 900px
- Money: 52px monospace tabular figure; every other number in the screen matches that grid
- Time: horizontal five-node rail with real timestamps under the money; live window as a meter
- Split: a bar that shows where the money goes *before* the click, so approval is not a leap
- Status: mono chip, not a colored pill; accent reserved for the live path and the primary action
- Interaction: Approve opens a confirm modal stating it is final and that approval has no admin key

### Trade-offs
- Strong at: video at 1080p, glanceable balance, credible "real crypto product" feel
- Weak at: fewer patterns per screen than 002, so dense terms need a second page later

### Best for
The demo recording, and the public site after it.
