#!/usr/bin/env bash
# Demo puppet-master: plays the freelancer and the arbiter so one person can record
# the whole escrow flow with a single wallet (the client).
#
#   ./scripts/demo.sh setup                 create the recording client account
#   ./scripts/demo.sh status <id>           one-line status
#   ./scripts/demo.sh freelancer <id>       accept + submit, the freelancer's side
#   ./scripts/demo.sh evidence <id>         both parties submit dispute evidence
#   ./scripts/demo.sh arbiter <id> <pct>    arbiter decides the split
#   ./scripts/demo.sh withdraw <who>        pull an allocation to a wallet
#   ./scripts/demo.sh reset                 list what is still locked/releasable
#
# Every call is idempotent-ish: if the project is already past a step, it says so
# instead of failing, so a botched take does not force a new escrow.

set -euo pipefail

export NEAR_TESTNET_RPC="${NEAR_TESTNET_RPC:-https://test.rpc.fastnear.com}"
export NODE_OPTIONS="${NODE_OPTIONS:---dns-result-order=ipv4first}"

CONTRACT="${CONTRACT:-nescrow-escrow.testnet}"
CLIENT="${CLIENT:-nescrow-rec-client.testnet}"
FREELANCER="${FREELANCER:-nescrow-freelancer.testnet}"
ARBITER="${ARBITER:-nescrow-arbiter.testnet}"

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }
ok()  { printf '  \033[32m✓\033[0m %s\n' "$*"; }
info(){ printf '  \033[2m·\033[0m %s\n' "$*"; }

gas_default=30000000000000
gas_withdraw=100000000000000

call() { # account method args [deposit] [gas]
  local account="$1" method="$2" args="$3" deposit="${4:-0}" gas="${5:-$gas_default}"
  near call "$CONTRACT" "$method" "$args" --accountId "$account" --deposit "$deposit" --gas "$gas" 2>&1 \
    | grep -E 'EVENT_JSON|Failure|panicked|Transaction Id' || true
}

field() { # json-ish output, field name
  grep -oE "\"$2\":\"[^\"]*\"" <<<"$1" | head -1 | cut -d'"' -f4
}

status_of() {
  near view "$CONTRACT" get_project "{\"project_id\":$1}" 2>/dev/null | grep -oE "status: '[A-Za-z]+'" | cut -d\' -f2
}

cmd_setup() {
  say "Creating the recording client account"
  if near state "$CLIENT" >/dev/null 2>&1; then
    ok "$CLIENT already exists"
  else
    near create-account "$CLIENT" --useFaucet --initialBalance 10 --networkId testnet >/dev/null
    ok "$CLIENT created with 10 NEAR"
  fi
  near state "$CLIENT" 2>/dev/null | grep -E 'formattedAmount' || true
}

cmd_status() {
  local id="$1"
  say "Project #$id"
  near view "$CONTRACT" get_project "{\"project_id\":$id}" 2>/dev/null \
    | grep -E "status:|amount:|title:|revision_count:|work_deadline:|review_deadline:" || info "not found"
}

cmd_freelancer() {
  local id="$1"
  local st; st="$(status_of "$id")"
  say "Freelancer acts on #$id (currently: $st)"

  if [[ "$st" == "Funded" ]]; then
    call "$FREELANCER" accept_project "{\"project_id\":$id}"
    ok "accepted"
  else
    info "already past acceptance ($st)"
  fi

  st="$(status_of "$id")"
  if [[ "$st" == "Accepted" || "$st" == "RevisionRequested" ]]; then
    call "$FREELANCER" submit_deliverable \
      "{\"project_id\":$id,\"link\":\"https://example.com/logo-v1\",\"note\":\"three concepts and the final SVG\"}"
    ok "delivered"
  else
    info "not in a state where delivery is allowed ($st)"
    return 1
  fi
}

cmd_evidence() {
  local id="$1"
  say "Both parties submit evidence on #$id"
  call "$CLIENT"     submit_evidence "{\"project_id\":$id,\"link\":\"https://example.com/brief\",\"note\":\"sections 3 and 4 are missing\"}" || true
  ok "client evidence submitted"
  call "$FREELANCER" submit_evidence "{\"project_id\":$id,\"link\":\"https://example.com/scope\",\"note\":\"the signed scope had two sections\"}" || true
  ok "freelancer evidence submitted"
}

cmd_arbiter() {
  local id="$1" pct="${2:-70}"
  say "Arbiter decides #$id at $pct% to the freelancer"
  call "$ARBITER" arbiter_resolve "{\"project_id\":$id,\"freelancer_percent\":$pct}"
  ok "decided"
}

cmd_withdraw() {
  local who="${1:-$FREELANCER}"
  say "Withdrawing $who"
  call "$who" withdraw '{}' 0 "$gas_withdraw"
  ok "withdrawn"
}

cmd_reset() {
  say "Current contract state"
  near view "$CONTRACT" get_fund_invariants 2>/dev/null | grep -E "locked:|claimable:|ok:" || true
  local n; n="$(near view "$CONTRACT" get_project_count 2>/dev/null | tail -1 | tr -d "'")"
  for ((i=0; i<n; i++)); do
    printf '  #%s %s\n' "$i" "$(status_of "$i")"
  done
}

case "${1:-}" in
  setup)     cmd_setup ;;
  status)    cmd_status "${2:?project id}" ;;
  freelancer) cmd_freelancer "${2:?project id}" ;;
  evidence)  cmd_evidence "${2:?project id}" ;;
  arbiter)   cmd_arbiter "${2:?project id}" "${3:-70}" ;;
  withdraw)  cmd_withdraw "${2:-$FREELANCER}" ;;
  reset)     cmd_reset ;;
  *) sed -n '2,14p' "$0"; exit 1 ;;
esac
