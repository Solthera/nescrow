# Demo script

Everything below runs against the deployed contract on testnet. Two accounts act the
client, one the freelancer, one the arbiter — so the whole flow can be shown without
switching wallets more than once.

## Before recording (10 minutes)

1. `cd app && npm run dev` — the app runs at http://localhost:5173
2. Fund the recording accounts so the numbers are clean:
   `near create-account nescrow-rec-client.testnet --useFaucet --initialBalance 10`
   (worth doing: a fresh client account means `projects created` and the dashboard
   start at zero rather than showing test debris)
3. Export every credential to where the CLI can read it, and export the RPC:
   `export NEAR_TESTNET_RPC=https://test.rpc.fastnear.com`
   `export NODE_OPTIONS=--dns-result-order=ipv4first`
4. Open on the laptop: the landing page, the app, and `terminal` with the CLI
   commands below pasted into a file so nothing is typed live.
5. Hide the browser bookmarks bar, use a 1440×900 window, 100% zoom.

## Scene 1 — the claim (0:00–0:25)

Landing page. Read the hero, then point at the right-hand panel:

> "That number is not a mockup — it is the contract's live balance, read from
> testnet. Zero fees, no owner key that can move escrow."

Scroll to **what it cannot do**. This is the differentiator; do not skip it.

## Scene 2 — fund a job (0:25–1:05)

`/app` → **New escrow**. Fill:

| Field | Value |
|---|---|
| Freelancer | `nescrow-freelancer.testnet` |
| Arbiter | `nescrow-arbiter.testnet` |
| Title | Logo for a Berlin client |
| Amount | 1 |
| Deadline | 7 days out |

Say: "One transaction. The escrow is the deposit — there is no second transfer, and
no amount field to disagree about later."

Sign, land on `/p/:id`. Status **Funded**, meter counting the 48-hour accept window.

## Scene 3 — the freelancer accepts (1:05–1:40)

Terminal:

```
near call nescrow-escrow.testnet accept_project '{"project_id":N}' --accountId nescrow-freelancer.testnet --gas 30000000000000
near call nescrow-escrow.testnet submit_deliverable '{"project_id":N,"link":"https://example.com/logo-v1","note":"three concepts and the final SVG"}' --accountId nescrow-freelancer.testnet --gas 30000000000000
```

Do **not** refresh the browser. The row updates on its own within 15 seconds:

> "No refresh — the app polls the chain. Transaction success is not UI success here;
> every write is followed by a refetch."

Rail moves to **Delivered**, meter switches to the 72-hour review window.

## Scene 4 — approve and withdraw (1:40–2:15)

Click **Approve · 1 NEAR**, then read the confirmation aloud:

> "It says final, because it is. The contract has no admin key to reverse this."

Sign. Split bar becomes `settled — freelancer 1.0000 / client 0.0000`.

Then show the payout is *pull*, not push — terminal:

```
near call nescrow-escrow.testnet withdraw '{}' --accountId nescrow-freelancer.testnet --gas 100000000000000
```

> "The outer transaction has a zero deposit by design. The transfer happens in the
> callback receipt — which is why a failed payment can never roll back a settlement."

## Scene 5 — the dispute path (2:15–3:00)

Create a second escrow for 2 NEAR, accept and submit as above, then in the app click
**Open dispute**. Then in the terminal:

```
near call nescrow-escrow.testnet submit_evidence '{"project_id":M,"link":"https://example.com/brief","note":"sections 3 and 4 missing"}' --accountId nescrow-client.testnet --gas 30000000000000
near call nescrow-escrow.testnet submit_evidence '{"project_id":M,"link":"https://example.com/scope","note":"the scope said two sections"}' --accountId nescrow-freelancer.testnet --gas 30000000000000
near call nescrow-escrow.testnet arbiter_resolve '{"project_id":M,"freelancer_percent":70}' --accountId nescrow-arbiter.testnet --gas 30000000000000
```

Show the decision card: 70/30 → 1.4000 / 0.6000, decided by arbiter.

## Scene 6 — the honest limitation (3:00–3:20)

Scroll to the known limitation in the README, or say it:

> "If the arbiter goes silent for seven days, funds stay locked. We chose that over
> giving an operator the power to split them. It is a documented limitation, not an
> oversight."

Close on `nescrow-escrow.testnet` in the explorer.

## The numbers that must appear

| Moment | Value |
|---|---|
| Landing, escrowed | increases by exactly the escrow you funded |
| Scene 4 split | `freelancer 1.0000` / `client 0.0000` |
| Scene 5 split | `freelancer 1.4000` / `client 0.6000` |
| Contract health, solvency | `balanced` at every point |

If any of those is off, stop the recording — the state is the product.

## What not to show

- The CLI output for `withdraw` on a second attempt: it is the `E-18` guard working,
  but it needs 10 seconds of explanation that kills the pace.
- The `nescrow-demo.testnet` account. It exists from an earlier bad build; README
  explains it, the video does not need to.
