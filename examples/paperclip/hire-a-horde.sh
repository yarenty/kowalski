#!/usr/bin/env bash
# Hire a horde as a Paperclip employee: the `process` adapter runs this script on each heartbeat.
# It takes the oldest issue assigned to the agent, runs the horde on it, posts the answers as a
# comment and marks the issue done (blocked when the run fails or waits for an approval).
#
#   hire-a-horde.sh <horde-id> [form-field]
#
#   hire-a-horde.sh url-summarizer                  # issue text goes in as the prompt
#   hire-a-horde.sh spreadsheet-analyst questions   # issue text fills the horde's form field
#
# Paperclip sets PAPERCLIP_API_URL, PAPERCLIP_API_KEY, PAPERCLIP_COMPANY_ID, PAPERCLIP_AGENT_ID and
# PAPERCLIP_RUN_ID. Needs a running kowalski (KOWALSKI_URL, default http://127.0.0.1:3456, the
# server's default bind; KOWALSKI_TOKEN when it runs with --auth), plus curl and python3.
set -euo pipefail

[ $# -ge 1 ] || { echo "usage: $0 <horde-id> [form-field]" >&2; exit 2; }
HORDE="$1"
FIELD="${2:-}"
URL="${KOWALSKI_URL:-http://127.0.0.1:3456}"

kw() {
  if [ -n "${KOWALSKI_TOKEN:-}" ]; then
    curl -sf -H "authorization: Bearer $KOWALSKI_TOKEN" "$@"
  else
    curl -sf "$@"
  fi
}
pc() {
  curl -sf -H "authorization: Bearer $PAPERCLIP_API_KEY" -H "x-paperclip-run-id: $PAPERCLIP_RUN_ID" \
    -H 'content-type: application/json' "$@"
}
json() { python3 -c "$1" "${@:2}"; }

issues=$(pc "$PAPERCLIP_API_URL/api/companies/$PAPERCLIP_COMPANY_ID/issues?assigneeAgentId=$PAPERCLIP_AGENT_ID&status=todo,in_progress")
issue=$(json '
import json, sys
data = json.loads(sys.argv[1])
issues = data if isinstance(data, list) else data.get("issues", data.get("items", []))
issues.sort(key=lambda i: i.get("createdAt", ""))
if issues:
    i = issues[0]
    print(i["id"], json.dumps((i.get("title", "") + "\n\n" + (i.get("description") or "")).strip()), sep="\t")
' "$issues")
[ -n "$issue" ] || { echo "No assigned issues; nothing to do."; exit 0; }
IFS=$'\t' read -r ISSUE TASK <<<"$issue"
echo "issue $ISSUE"

pc -X POST -d "{\"agentId\":\"$PAPERCLIP_AGENT_ID\",\"expectedStatuses\":[\"todo\",\"in_progress\"]}" \
  "$PAPERCLIP_API_URL/api/issues/$ISSUE/checkout" >/dev/null || echo "checkout refused; working on it anyway" >&2

finish() {  # status, comment
  pc -X PATCH -d "$(json 'import json, sys; print(json.dumps({"status": sys.argv[1], "comment": sys.argv[2]}))' "$1" "$2")" \
    "$PAPERCLIP_API_URL/api/issues/$ISSUE" >/dev/null
}

body=$(json '
import json, sys
task, field = json.loads(sys.argv[1]), sys.argv[2]
print(json.dumps({"form_answers": {field: task}} if field else {"prompt": task}))
' "$TASK" "$FIELD")
run=$(kw -H 'content-type: application/json' -d "$body" "$URL/api/hordes/$HORDE/run" \
  | json 'import json, sys; print(json.load(sys.stdin)["run"]["run_id"])' 2>/dev/null) \
  || { finish blocked "Could not start the kowalski horde \`$HORDE\` at $URL."; exit 1; }
echo "run $run"

while :; do
  state=$(kw "$URL/api/hordes/$HORDE/runs/$run" | json '
import json, sys
data = json.load(sys.stdin)
run = data.get("run", data)
done = [e for e in run["events"] if e.get("kind") == "run_finished"]
print(run["status"], json.dumps((done[-1].get("handoff_markdown") if done else None) or ""), sep="\t")')
  IFS=$'\t' read -r status handoff <<<"$state"
  case "$status" in
    running|queued|pending) sleep 3 ;;
    *) break ;;
  esac
done

answer=$(json 'import json, sys; print(json.loads(sys.argv[1]))' "$handoff")
case "$status" in
  completed)
    [ -n "$answer" ] || answer="The run finished without a hand-off; its files are in the horde's output folder."
    echo "$answer"
    finish done "$answer"
    ;;
  awaiting_input)
    finish blocked "Kowalski run \`$run\` is waiting for an approval: open the horde \`$HORDE\` in kowalski."
    exit 1
    ;;
  *)
    finish blocked "Kowalski run \`$run\` ended: $status."
    exit 1
    ;;
esac
