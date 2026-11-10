#!/usr/bin/env bash
# End-to-end check of the deployed testnet contracts with real accounts.
#
# Creates fresh wallets, funds them with Friendbot, adds USDC trustlines, buys
# test USDC on the testnet DEX (Circle's faucet is a captcha-protected web
# form, so a script cannot use it), then drives real jobs through every path
# and checks exact USDC balances after each one.
#
# Usage: scripts/e2e-testnet.sh
#
#   POLICY_OWNER  Stellar CLI identity owning the evaluation policy (default:
#                 deployer). Used to add temporary signers for the policy
#                 scenario and remove them again. Set POLICY_OWNER= to skip it.
#   KEEP_WALLETS  set to 1 to keep the generated identities afterwards.
#
# Contract IDs come from deployments/testnet.json. Needs the Stellar CLI, jq
# and curl. Takes about five minutes, mostly waiting for ledgers.
set -euo pipefail
cd "$(dirname "$0")/.."

network=testnet
manifest=deployments/testnet.json
kernel="$(jq -r '.contracts["pactlane-commerce"].contractId' "$manifest")"
policy="$(jq -r '.contracts["pactlane-evaluation-policy"].contractId' "$manifest")"
usdc="$(jq -r '.contracts["pactlane-commerce"].constructorArgs.token' "$manifest")"
usdc_asset="USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
owner="${POLICY_OWNER-deployer}"

run_id="e2e-$(date +%s)"
roles=(client provider evaluator signer)
passed=0
failed=0
signer_added=

say() { printf '\n== %s\n' "$*"; }
ok() { printf '  ok   %s\n' "$*"; passed=$((passed + 1)); }
bad() { printf '  FAIL %s\n' "$*" >&2; failed=$((failed + 1)); }
id() { echo "$run_id-$1"; }
addr() { stellar keys address "$(id "$1")"; }
# h <text>: a SHA-256 commitment as 64 hex characters.
h() { printf '%s' "$1" | sha256sum | cut -d' ' -f1; }

cleanup() {
  if [ -n "$owner" ] && [ -n "$signer_added" ]; then
    for role in signer provider; do
      stellar contract invoke --id "$policy" --source-account "$owner" --network "$network" \
        -- remove_signer --signer "$(addr "$role")" >/dev/null 2>&1 || true
    done
  fi
  if [ "${KEEP_WALLETS:-0}" != 1 ]; then
    for role in "${roles[@]}"; do stellar keys rm --force "$(id "$role")" >/dev/null 2>&1 || true; done
  fi
}
trap cleanup EXIT

# Resolves a role name to its identity; anything else is used as given.
source_of() {
  case "$1" in client | provider | evaluator | signer) id "$1" ;; *) echo "$1" ;; esac
}

# invoke <source> <contract> <function> [args...]: prints the result.
invoke() {
  local source contract="$2"
  source="$(source_of "$1")"
  shift 2
  stellar contract invoke --id "$contract" --source-account "$source" --network "$network" \
    -- "$@" 2>/dev/null | tr -d '"'
}

# view <function> [args...] on the kernel, simulated only.
view() {
  stellar contract invoke --id "$kernel" --source-account "$(id client)" --network "$network" \
    --send=no -- "$@" 2>/dev/null
}

# token_balance <address>: USDC base units held.
token_balance() {
  stellar contract invoke --id "$usdc" --source-account "$(id client)" --network "$network" \
    --send=no -- balance --id "$1" 2>/dev/null | tr -d '"'
}
balance() { token_balance "$(addr "$1")"; }

# expect_refused <label> <expected substring> <source> <contract> <function> [args...]
expect_refused() {
  local label="$1" want="$2" source contract="$4" err
  source="$(source_of "$3")"
  shift 4
  if err="$(stellar contract invoke --id "$contract" --source-account "$source" \
    --network "$network" -- "$@" 2>&1 >/dev/null)"; then
    bad "$label: succeeded, expected refusal"
  elif grep -q -- "$want" <<<"$err"; then
    ok "$label refused ($want)"
  else
    bad "$label: refused, but not with $want: $(grep -m1 -i error <<<"$err")"
  fi
}

check_eq() { if [ "$2" = "$3" ]; then ok "$1 = $3"; else bad "$1 is $2, expected $3"; fi; }

state_of() { view get_job --id "$1" | jq -r .state; }

# create_job <evaluator address> <seconds until expiry> [--no-provider]: prints the job ID.
# The CLI takes Option<_> arguments as JSON, so the provider is a quoted string.
create_job() {
  local evaluator="$1" ttl="$2" provider_arg=(--provider "\"$(addr provider)\"")
  [ "${3:-}" = --no-provider ] && provider_arg=()
  invoke client "$kernel" create_job --client "$(addr client)" "${provider_arg[@]}" \
    --evaluator "$evaluator" --expires_at "$(($(date +%s) + ttl))" \
    --spec_hash "$(h "$run_id-$RANDOM")"
}

# funded_job <evaluator address> <seconds until expiry> <budget>: prints the job ID.
funded_job() {
  local job
  job="$(create_job "$1" "$2")"
  invoke client "$kernel" set_budget --id "$job" --actor "$(addr client)" --amount "$3" >/dev/null
  invoke client "$kernel" fund --id "$job" --expected_budget "$3" >/dev/null
  echo "$job"
}

say "Wallets ($run_id)"
for role in "${roles[@]}"; do
  stellar keys generate "$(id "$role")" --network "$network" --fund >/dev/null
  ok "$role $(addr "$role") funded by Friendbot"
done

say "USDC"
for role in client provider; do
  stellar tx new change-trust --source-account "$(id "$role")" --network "$network" \
    --line "$usdc_asset" >/dev/null 2>&1
  ok "$role trusts $usdc_asset"
done
# 100 XLM for at least 10 USDC (amounts in stroops / base units).
stellar tx new path-payment-strict-send --source-account "$(id client)" --network "$network" \
  --send-asset native --send-amount 1000000000 --destination "$(addr client)" \
  --dest-asset "$usdc_asset" --dest-min 100000000 >/dev/null 2>&1
start="$(balance client)"
if [ "$start" -ge 100000000 ]; then ok "client bought $start base units of USDC"; else
  bad "client holds only $start base units of USDC"
  exit 1
fi

budget=10000000 # 1 USDC
escrow_before="$(token_balance "$kernel")"
count_before="$(view job_count)"

say "1. Paid: create, propose, fund, submit, complete"
paid="$(create_job "$(addr evaluator)" 3600)"
ok "job $paid created"
invoke provider "$kernel" set_budget --id "$paid" --actor "$(addr provider)" --amount "$budget" >/dev/null
invoke client "$kernel" fund --id "$paid" --expected_budget "$budget" >/dev/null
check_eq "client after funding" "$(balance client)" "$((start - budget))"
invoke provider "$kernel" submit --id "$paid" --work_hash "$(h work)" >/dev/null
invoke evaluator "$kernel" complete --id "$paid" --reason "$(h ok)" >/dev/null
check_eq "job $paid state" "$(state_of "$paid")" 3
check_eq "provider paid" "$(balance provider)" "$budget"

say "2. Rejected by the evaluator: client refunded"
rejected="$(funded_job "$(addr evaluator)" 3600 "$budget")"
invoke evaluator "$kernel" reject --id "$rejected" >/dev/null
check_eq "job $rejected state" "$(state_of "$rejected")" 4
check_eq "client after refund" "$(balance client)" "$((start - budget))"

say "3. Cancelled while open: nothing moves"
cancelled="$(create_job "$(addr evaluator)" 3600 --no-provider)"
invoke client "$kernel" set_budget --id "$cancelled" --actor "$(addr client)" --amount "$budget" >/dev/null
expect_refused "fund without a provider" "#9" client "$kernel" fund --id "$cancelled" --expected_budget "$budget"
invoke client "$kernel" reject --id "$cancelled" >/dev/null
check_eq "job $cancelled state" "$(state_of "$cancelled")" 4
check_eq "client after cancel" "$(balance client)" "$((start - budget))"

say "4. Refusals"
race="$(create_job "$(addr evaluator)" 3600)"
invoke client "$kernel" set_budget --id "$race" --actor "$(addr client)" --amount "$budget" >/dev/null
invoke provider "$kernel" set_budget --id "$race" --actor "$(addr provider)" --amount "$((budget * 2))" >/dev/null
expect_refused "fund at a stale budget" "#8" client "$kernel" fund --id "$race" --expected_budget "$budget"
# A job waiting for its evaluator, so only the signature is wrong. The CLI
# cannot even build these calls: they need the evaluator's signature.
pending="$(funded_job "$(addr evaluator)" 3600 "$budget")"
invoke provider "$kernel" submit --id "$pending" --work_hash "$(h w)" >/dev/null
expect_refused "provider approving its own work (evaluator did not sign)" "Missing signing key" \
  provider "$kernel" complete --id "$pending" --reason "$(h x)"
expect_refused "client rejecting funded work (evaluator did not sign)" "Missing signing key" \
  client "$kernel" reject --id "$pending"
expect_refused "funding the same job twice" "#2" client "$kernel" fund --id "$pending" --expected_budget "$budget"
expect_refused "submitting twice" "#2" provider "$kernel" submit --id "$pending" --work_hash "$(h w)"
expect_refused "completing a paid job again" "#2" evaluator "$kernel" complete --id "$paid" --reason "$(h x)"
expect_refused "refunding a paid job" "#2" client "$kernel" claim_refund --id "$paid"
expect_refused "rejecting a cancelled job" "#2" client "$kernel" reject --id "$cancelled"
expect_refused "proposing a budget on a paid job" "#2" client "$kernel" set_budget --id "$paid" \
  --actor "$(addr client)" --amount "$budget"
expect_refused "a budget proposed by an outsider" "#3" evaluator "$kernel" set_budget --id "$race" \
  --actor "$(addr evaluator)" --amount "$budget"
expect_refused "a zero budget" "#7" client "$kernel" set_budget --id "$race" --actor "$(addr client)" --amount 0
expect_refused "an expiry beyond 90 days" "#4" client "$kernel" create_job --client "$(addr client)" \
  --evaluator "$(addr evaluator)" --expires_at "$(($(date +%s) + 91 * 86400))" --spec_hash "$(h s)"
expect_refused "a client evaluating its own job" "#11" client "$kernel" create_job --client "$(addr client)" \
  --evaluator "$(addr client)" --expires_at "$(($(date +%s) + 3600))" --spec_hash "$(h s)"
expect_refused "replacing a provider" "#10" client "$kernel" set_provider --id "$race" --provider "$(addr signer)"
expect_refused "unknown job" "#1" client "$kernel" get_job --id 999999999
invoke evaluator "$kernel" complete --id "$pending" --reason "$(h ok)" >/dev/null
check_eq "job $pending completed by its real evaluator" "$(state_of "$pending")" 3
invoke client "$kernel" reject --id "$race" >/dev/null

say "4b. Provider assigned after creation, budget proposed by the provider"
late="$(create_job "$(addr evaluator)" 3600 --no-provider)"
invoke client "$kernel" set_provider --id "$late" --provider "$(addr provider)" >/dev/null
check_eq "job $late provider" "$(view get_job --id "$late" | jq -r .provider)" "$(addr provider)"
invoke provider "$kernel" set_budget --id "$late" --actor "$(addr provider)" --amount "$((budget / 2))" >/dev/null
invoke client "$kernel" fund --id "$late" --expected_budget "$((budget / 2))" >/dev/null
invoke provider "$kernel" submit --id "$late" --work_hash "$(h late)" >/dev/null
invoke evaluator "$kernel" complete --id "$late" --reason "$(h ok)" >/dev/null
check_eq "job $late state" "$(state_of "$late")" 3

# Paid out so far: paid, pending and late.
spent=$((2 * budget + budget / 2))

if [ -n "$owner" ]; then
  say "5. Settled through the evaluation policy"
  invoke "$owner" "$policy" add_signer --signer "$(addr signer)" >/dev/null
  invoke "$owner" "$policy" add_signer --signer "$(addr provider)" >/dev/null
  signer_added=1
  ok "signers $(addr signer) and the provider added"
  via="$(funded_job "$policy" 3600 "$budget")"
  invoke provider "$kernel" submit --id "$via" --work_hash "$(h work2)" >/dev/null
  expect_refused "settle by a non-signer" "#101" evaluator "$policy" complete \
    --signer "$(addr evaluator)" --id "$via" --reason "$(h ok)"
  expect_refused "a signer approving its own work" "#103" provider "$policy" complete \
    --signer "$(addr provider)" --id "$via" --reason "$(h ok)"
  invoke signer "$policy" complete --signer "$(addr signer)" --id "$via" --reason "$(h ok)" >/dev/null
  check_eq "job $via state" "$(state_of "$via")" 3
  spent=$((spent + budget))
  check_eq "provider paid for four jobs" "$(balance provider)" "$spent"
  invoke "$owner" "$policy" remove_signer --signer "$(addr signer)" >/dev/null
  invoke "$owner" "$policy" remove_signer --signer "$(addr provider)" >/dev/null
  signer_added=
  check_eq "removed signer is_signer" "$(invoke signer "$policy" is_signer --address "$(addr signer)")" false
fi

say "6. Expired: anyone claims the refund"
expired="$(funded_job "$(addr evaluator)" 60 "$budget")"
lapsed="$(funded_job "$(addr evaluator)" 60 "$budget")"
expect_refused "refund before expiry" "#6" provider "$kernel" claim_refund --id "$expired"
# Wait out the later of the two deadlines.
deadline="$(view get_job --id "$lapsed" | jq -r .expires_at)"
while [ "$(date +%s)" -le "$((deadline + 10))" ]; do sleep 5; done
# Sent by the provider, who gains nothing: the money can only go to the client.
invoke provider "$kernel" claim_refund --id "$expired" >/dev/null
check_eq "job $expired state" "$(state_of "$expired")" 5
expect_refused "evaluator rejecting after expiry" "#5" evaluator "$kernel" reject --id "$lapsed"
expect_refused "submitting after expiry" "#5" provider "$kernel" submit --id "$lapsed" --work_hash "$(h w)"
expect_refused "refunding twice" "#2" provider "$kernel" claim_refund --id "$expired"
invoke signer "$kernel" claim_refund --id "$lapsed" >/dev/null
check_eq "job $lapsed refunded by an unrelated account" "$(state_of "$lapsed")" 5
invoke signer "$kernel" extend_ttl --id "$paid" >/dev/null
ok "extend_ttl on a finished job by an unrelated account"
check_eq "client after all jobs" "$(balance client)" "$((start - spent))"
check_eq "provider after all jobs" "$(balance provider)" "$spent"

say "Views"
count_after="$(view job_count)"
created=$((count_after - count_before))
if [ "$created" -ge 9 ]; then ok "job_count rose by $created"; else bad "job_count rose by $created"; fi
listed="$(view get_jobs --start "$paid" --limit 3 | jq -r '[.[].id] | join(",")')"
check_eq "get_jobs from $paid" "$listed" "$paid,$((paid + 1)),$((paid + 2))"
check_eq "get_jobs caps a page at 50" "$(view get_jobs --start 1 --limit 1000 | jq length)" \
  "$((count_after < 50 ? count_after : 50))"
check_eq "get_jobs past the end" "$(view get_jobs --start 999999999 --limit 5 | jq length)" 0
check_eq "kernel escrow back to its starting balance" "$(token_balance "$kernel")" "$escrow_before"

printf '\n%d passed, %d failed\n' "$passed" "$failed"
[ "$failed" -eq 0 ]
