#!/bin/bash
#
# Build or deploy one of the Cloudflare-hosted frontends from the repository root.
#
#   bash script/website/cloudflare_app.sh <build|deploy>
#
# Every Cloudflare Worker builds from this repository, and nothing in the Workers
# Builds environment names the project, so the app comes from ARTCRAFT_APP. The
# app-specific npm scripts set it inline and are the commands to configure:
#
#   artcraft-website (default)  getartcraft.com, root wrangler.jsonc
#                               npm run build:website / deploy:website
#   artcraft-webapp             app.getartcraft.com, frontend/apps/artcraft-webapp/wrangler.jsonc
#                               npm run build:webapp / deploy:webapp
#   artcraft-dashboard          dashboard.artcraft.team, frontend/apps/artcraft-dashboard/wrangler.jsonc
#                               npm run build:dashboard / deploy:dashboard
#   storyteller-docs            storyteller-docs.artcraft.team, frontend/api_docs/wrangler.jsonc
#                               npm run build:docs / deploy:docs
#
# The bare `npm run build` / `npm run deploy` pair falls back to the website and
# needs an ARTCRAFT_APP build variable. Only the website builds
# artcraft-website-next, so only it needs the website's build secrets
# (LAUNCHER_CATALOG_SIGNING_KEY, and the optional LINUX_SIGNING_KEY); the webapp
# and dashboard read neither.

set -euo pipefail

ACTION="${1:-}"
APP="${ARTCRAFT_APP:-artcraft-website}"

cd "$(dirname "$0")/../.."

WRANGLER_PREFIX=()
WRANGLER_CMD="deploy"
if [[ "${ACTION}" == "deploy" && -n "${WORKERS_CI_BRANCH:-}" && "${WORKERS_CI_BRANCH}" != "main" ]]; then
  WRANGLER_PREFIX=(env -u WRANGLER_CI_MATCH_TAG)
  WRANGLER_CMD="preview"
fi

case "${APP}:${ACTION}" in
  artcraft-website:build)
    npm --prefix frontend/apps/artcraft-website-next ci --ignore-scripts
    npm --prefix frontend/apps/artcraft-website-next run build:cloudflare
    ;;
  artcraft-website:deploy)
    "${WRANGLER_PREFIX[@]}" npx --yes wrangler@4 ${WRANGLER_CMD}
    ;;
  artcraft-webapp:build)
    npm --prefix frontend ci --ignore-scripts
    npm --prefix frontend run build:webapp
    ;;
  artcraft-webapp:deploy)
    "${WRANGLER_PREFIX[@]}" npx --yes wrangler@4 ${WRANGLER_CMD} --config frontend/apps/artcraft-webapp/wrangler.jsonc
    ;;
  artcraft-dashboard:build)
    # Standalone pnpm app outside the Nx/npm workspace; the script installs and
    # builds inside its own folder.
    bash frontend/apps/artcraft-dashboard/script/build_cloudflare.sh
    ;;
  artcraft-dashboard:deploy)
    "${WRANGLER_PREFIX[@]}" npx --yes wrangler@4 ${WRANGLER_CMD} --config frontend/apps/artcraft-dashboard/wrangler.jsonc
    ;;
  storyteller-docs:build)
    # The OpenAPI document comes from `cargo run --bin docs-cli`, which Workers
    # Builds cannot run, so the artifacts are committed: this build only checks
    # that they are present and that the document still parses.
    for required in frontend/api_docs/public/index.html frontend/api_docs/public/api.json; do
      if [[ ! -f "${required}" ]]; then
        echo "Missing ${required}; regenerate with: bash script/build_api_docs.sh" >&2
        exit 1
      fi
    done
    node --input-type=module -e '
      const { readFileSync } = await import("node:fs");
      const spec = JSON.parse(readFileSync("frontend/api_docs/public/api.json", "utf8"));
      const paths = Object.keys(spec.paths ?? {});
      if (paths.length === 0) throw new Error("api.json contains no paths");
      console.log(`api.json: ${paths.length} paths (${spec.info?.title ?? "untitled"})`);
    '
    ;;
  storyteller-docs:deploy)
    "${WRANGLER_PREFIX[@]}" npx --yes wrangler@4 ${WRANGLER_CMD} --config frontend/api_docs/wrangler.jsonc
    ;;
  *)
    echo "Usage: ARTCRAFT_APP=<artcraft-website|artcraft-webapp|artcraft-dashboard|storyteller-docs> $0 <build|deploy>" >&2
    echo "Got ARTCRAFT_APP='${APP}', action='${ACTION}'." >&2
    exit 1
    ;;
esac
