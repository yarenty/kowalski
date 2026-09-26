#!/usr/bin/env bash
# Ask the Spreadsheet analyst questions from a terminal and print the answers.
#
#   ./demo.sh "Who spent the most in total across the orders?" "How many people live in each city?"
#
# Needs a running kowalski with tableski connected (KOWALSKI_URL, default http://127.0.0.1:3456;
# KOWALSKI_TOKEN when the server runs with --auth), plus curl and python3.
set -euo pipefail

URL="${KOWALSKI_URL:-http://127.0.0.1:3456}"
api() {
  if [ -n "${KOWALSKI_TOKEN:-}" ]; then
    curl -sf -H "authorization: Bearer $KOWALSKI_TOKEN" "$@"
  else
    curl -sf "$@"
  fi
}
[ $# -gt 0 ] || { echo "usage: $0 \"question\" [\"question\" ...]" >&2; exit 2; }

body=$(python3 -c 'import json,sys; print(json.dumps({"form_answers": {"questions": "\n".join(sys.argv[1:]), "tables": ""}}))' "$@")
run=$(api -H 'content-type: application/json' -d "$body" "$URL/api/hordes/spreadsheet-analyst/run" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["run"]["run_id"])')
echo "run $run"

last=""
while :; do
  state=$(api "$URL/api/hordes/spreadsheet-analyst/runs" | python3 -c '
import json, sys
run = next(r for r in json.load(sys.stdin)["runs"] if r["run_id"] == sys.argv[1])
steps = " ".join(s["step"] + ":" + s["status"] for s in run.get("steps", []))
handoff = next((e["artifact"] for e in run["events"] if e.get("step") == "deliver" and e.get("artifact")), "")
print(run["status"], handoff, steps, sep="\t")' "$run")
  IFS=$'\t' read -r status handoff steps <<<"$state"
  [ "$steps" != "$last" ] && echo "  $steps" && last="$steps"
  case "$status" in
    running|queued|pending) sleep 3 ;;
    *) break ;;
  esac
done

if [ "$status" != "completed" ] || [ -z "$handoff" ]; then
  echo "run ended: $status" >&2
  exit 1
fi
echo
cat "$handoff"
echo
echo "Workbook: $(dirname "$handoff")/report.xlsx"
