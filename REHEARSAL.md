# Rehearsal — practise before recording

The demo has one actor (you, as the client) and two puppets played by a script. Practise
the script twice; only record on the third pass.

## 0. Setup, once

```bash
cd ~/Projects/NEARHACK
export PATH="$HOME/.npm-global/bin:$PATH"
export NEAR_TESTNET_RPC=https://test.rpc.fastnear.com
export NODE_OPTIONS=--dns-result-order=ipv4first

./scripts/demo.sh setup        # created nescrow-rec-client.testnet, 10 NEAR
./scripts/demo.sh reset        # what is on chain right now
```

App running: `cd app && npm run dev` → http://localhost:5173

Wallet: connect **nescrow-rec-client.testnet** in the browser. Its key is at
`~/.near-credentials/testnet/nescrow-rec-client.testnet.json` — import that private key
into Meteor (or MyNearWallet) once, so the recording never shows a wallet setup screen.

## 1. Dry run (no recording) — the A path

Do this with the screen recorder off, just to feel the timing. Terminal stays visible
for one thing only: the puppet commands.

| # | Where | Action | What you say (one line) | What must change on screen |
|---|---|---|---|---|
| 1 | `/` landing | point at the right panel | "That is the contract's live balance, not a mockup." | escrowed number matches what you fund |
| 2 | `/app` | click **New escrow** | "One transaction, no second transfer." | form appears, min escrow from config |
| 3 | form | freelancer `nescrow-freelancer.testnet`, arbiter `nescrow-arbiter.testnet`, title `Logo for a Berlin client`, amount `1`, deadline +7 days | "The escrow is the deposit." | button changes to "Waiting for the wallet…" |
| 4 | wallet popup | approve | — | lands on `/p/N`, status **Funded**, meter ~48h |
| 5 | terminal | `./scripts/demo.sh freelancer N` | "The freelancer's side, run for the camera." | — |
| 6 | browser, no refresh | wait ≤15 s | "No refresh — it polls the chain." | **Delivered**, meter switches to 72h review |
| 7 | browser | **Approve · 1 NEAR** | "It says final, because it is." | split bar → `settled · 1.0000 / 0.0000` |
| 8 | terminal | `./scripts/demo.sh withdraw nescrow-freelancer.testnet` | "Pull payment: the transfer is in the receipt." | claimable drops to 0 |

Stop here on the first practice. That is 8 steps and about 90 seconds of screen time.

## 2. Dry run — the B path (dispute)

New escrow, 2 NEAR. Then:

| # | Where | Action | What must change |
|---|---|---|---|
| 1 | browser | create → `./scripts/demo.sh freelancer M` | **Delivered** |
| 2 | browser | **Open dispute** | **Disputed**, arbiter window meter starts |
| 3 | terminal | `./scripts/demo.sh evidence M` | evidence rows appear in the dispute card |
| 4 | terminal | `./scripts/demo.sh arbiter M 70` | `settled · 1.4000 / 0.6000`, decided by arbiter |

## 3. The numbers that must appear

If any of these is wrong, stop — the state is the product, not the pixels.

| Moment | Value |
|---|---|---|
| split after approval (1 NEAR job) | `freelancer 1.0000` / `client 0.0000` |
| split after arbiter (2 NEAR job, 70%) | `freelancer 1.4000` / `client 0.6000` |
| contract health | `balanced` at every point |
| withdraw label | the **claimable** amount, not the escrow size |

## 4. Recovery — a botched take

The script refuses to repeat a step rather than failing, so:

```bash
./scripts/demo.sh status N     # where is this project
./scripts/demo.sh reset        # every project and its status
```

A project already `Released` cannot be re-approved. If the take is beyond saving simply
create another escrow — ids are cheap. Do not try to "fix" a project mid-recording.

## 5. Recording pass

Resolution 1440×900, 100% zoom, bookmarks bar hidden. Terminal font ≥ 14px, dark theme
to match the app. Record in one take if possible; the polling means nothing needs to be
staged.

Suggested order for the video itself: landing → create → freelancer puppet → approve →
withdraw receipt → dispute path → the known limitation → explorer link.

## 6. What not to put in the video

- A second `withdraw` attempt (that is the `E-18` guard working, but it costs 10 seconds
  of explanation).
- `nescrow-demo.testnet` — it is an old bad build, explained in the README.
- Wallet installation or faucet steps. Assume the viewer has a wallet.
