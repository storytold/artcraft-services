#!/bin/bash
# apply_metric_configs.sh
#
# Idempotently applies per-metric tag configurations and percentile-aggregation
# toggles to our Datadog account. Sibling to `apply_dashboards.sh` and
# `apply_monitors.sh`.
#
# **Run this before applying dashboards** that query Datadog distribution
# metrics with `p50:` / `p95:` / `p99:` time aggregators — the percentile
# aggregations have to exist on the metric side before any dashboard
# widget can query them.
#
# Each JSON file under `_metrics/datadog/metric_configs/` describes one
# metric. Schema:
#
#   {
#     "metric_name": "http.server.request.duration_ms",
#     "metric_type": "distribution",
#     "tags": ["env", "service", "host", ...],
#     "include_percentiles": true,
#     "_comment": "..."   // any underscore-prefixed key is ignored
#   }
#
# Apply behavior, per file:
#   - GET /api/v2/metrics/{name}/tags
#       200 → PATCH (update existing config)
#       404 → POST  (create new config)
#       other → fail
#
# Secrets:
#   secrets/datadog_key.txt           — DD-API-KEY        (required)
#   secrets/datadog_app_key.txt       — DD-APPLICATION-KEY (required)
#
# Flags:
#   --dry-run            Print what would happen; don't call the API.
#   --site <host>        Datadog site host. Default: api.datadoghq.com.
#   --dir <path>         Override directory.
#                        Default: _metrics/datadog/metric_configs

set -euo pipefail

# -------- defaults --------
DD_SITE="api.datadoghq.com"
CFG_DIR="_metrics/datadog/metric_configs"
API_KEY_FILE="secrets/datadog_key.txt"
APP_KEY_FILE="secrets/datadog_app_key.txt"
DRY_RUN=0

# -------- args --------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --site) DD_SITE="$2"; shift 2 ;;
    --dir)  CFG_DIR="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,38p' "$0"; exit 0 ;;
    *)
      echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done

BASE="https://${DD_SITE}"

# -------- preflight --------
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }

if [[ ! -s "$API_KEY_FILE" ]]; then
  echo "missing or empty API key file: $API_KEY_FILE" >&2
  exit 1
fi
if [[ ! -s "$APP_KEY_FILE" ]]; then
  cat >&2 <<EOF
Missing or empty application key file: $APP_KEY_FILE

The metric-config API requires BOTH a Datadog API key AND an Application key.
Generate an Application key at:
  https://app.datadoghq.com/organization-settings/application-keys
and save it as $APP_KEY_FILE (just the key, one line).
EOF
  exit 1
fi

DD_API_KEY=$(tr -d '[:space:]' < "$API_KEY_FILE")
DD_APP_KEY=$(tr -d '[:space:]' < "$APP_KEY_FILE")

if [[ ! -d "$CFG_DIR" ]]; then
  echo "no such directory: $CFG_DIR" >&2
  exit 1
fi

shopt -s nullglob
files=("$CFG_DIR"/*.json)
if [[ ${#files[@]} -eq 0 ]]; then
  echo "no metric-config JSON files in $CFG_DIR" >&2
  exit 0
fi

exit_code=0

for file in "${files[@]}"; do
  echo
  echo "==> $file"

  if ! jq empty "$file" 2>/dev/null; then
    echo "   invalid JSON, skipping" >&2
    exit_code=1
    continue
  fi

  metric_name=$(jq -r '.metric_name // empty' "$file")
  if [[ -z "$metric_name" ]]; then
    echo "   missing `metric_name`, skipping" >&2
    exit_code=1
    continue
  fi

  metric_type=$(jq -r '.metric_type // "distribution"' "$file")
  tags_json=$(jq -c '.tags // []' "$file")
  include_pct=$(jq -r '.include_percentiles // false' "$file")

  # Build POST and PATCH payloads separately. POST creates the config and
  # accepts `metric_type` (required to declare it as a distribution / count /
  # etc.). PATCH only accepts mutable attributes (`tags`, `include_percentiles`).
  post_payload=$(jq -n \
    --arg name "$metric_name" \
    --arg metric_type "$metric_type" \
    --argjson tags "$tags_json" \
    --argjson include_pct "$include_pct" \
    '{
      data: {
        type: "manage_tags",
        id: $name,
        attributes: {
          tags: $tags,
          include_percentiles: $include_pct,
          metric_type: $metric_type
        }
      }
    }')
  patch_payload=$(jq -n \
    --arg name "$metric_name" \
    --argjson tags "$tags_json" \
    --argjson include_pct "$include_pct" \
    '{
      data: {
        type: "manage_tags",
        id: $name,
        attributes: {
          tags: $tags,
          include_percentiles: $include_pct
        }
      }
    }')

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "   [dry-run] would upsert tag config for $metric_name"
    echo "$post_payload" | jq '.data.attributes'
    continue
  fi

  # Detect existing config.
  probe_file=$(mktemp)
  probe_status=$(curl -sS -o "$probe_file" -w "%{http_code}" \
    -X GET "${BASE}/api/v2/metrics/${metric_name}/tags" \
    -H "DD-API-KEY: ${DD_API_KEY}" -H "DD-APPLICATION-KEY: ${DD_APP_KEY}" \
    || echo "000")

  if [[ "$probe_status" = "200" ]]; then
    method="PATCH"
    echo "   updating existing tag config (include_percentiles=$include_pct)"
  elif [[ "$probe_status" = "404" ]]; then
    method="POST"
    echo "   creating tag config (include_percentiles=$include_pct)"
  else
    echo "   ✗ probe returned HTTP $probe_status"
    sed 's/^/      /' "$probe_file" >&2
    rm -f "$probe_file"
    exit_code=1
    continue
  fi
  rm -f "$probe_file"

  payload="$post_payload"
  [[ "$method" = "PATCH" ]] && payload="$patch_payload"

  response_file=$(mktemp)
  curl_status=$(curl -sS -o "$response_file" -w "%{http_code}" \
    -X "$method" "${BASE}/api/v2/metrics/${metric_name}/tags" \
    -H "Content-Type: application/json" \
    -H "DD-API-KEY: ${DD_API_KEY}" \
    -H "DD-APPLICATION-KEY: ${DD_APP_KEY}" \
    --data-binary "$payload" || echo "000")

  if [[ "$curl_status" -lt 200 ]] || [[ "$curl_status" -ge 300 ]]; then
    echo "   ✗ ${method} returned HTTP $curl_status"
    sed 's/^/      /' "$response_file" >&2
    exit_code=1
  else
    echo "   ✓ ${method} succeeded"
  fi
  rm -f "$response_file"
done

echo
if [[ "$exit_code" -eq 0 ]]; then
  echo "All metric configs applied."
else
  echo "Finished with errors. See output above."
fi
exit "$exit_code"
