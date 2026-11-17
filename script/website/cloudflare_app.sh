#!/bin/bash
#
# Build or deploy one of the Cloudflare-hosted frontends from the repository root.
#
#   bash script/website/cloudflare_app.sh <build|deploy>
#
# Both Cloudflare projects build from this repository with `npm run build` and
# `npm run deploy`. Nothing in the Workers Builds environment names the project,
# so each project sets the ARTCRAFT_APP build variable:
#
#   artcraft-website (default)  getartcraft.com, root wrangler.jsonc
#   artcraft-webapp             app.getartcraft.com, frontend/apps/artcraft-webapp/wrangler.jsonc
#
# The webapp never builds artcraft-website-next, so it needs none of the website's
# build secrets (LAUNCHER_CATALOG_SIGNING_KEY, and the optional LINUX_SIGNING_KEY).

set -euo pipefail

ACTION="${1:-}"
APP="${ARTCRAFT_APP:-artcraft-website}"

cd "$(dirname "$0")/../.."

case "${APP}:${ACTION}" in
  artcraft-website:build)
    npm --prefix frontend/apps/artcraft-website-next ci --ignore-scripts
    npm --prefix frontend/apps/artcraft-website-next run build:cloudflare
    ;;
  artcraft-website:deploy)
    npx --yes wrangler@4 deploy
    ;;
  artcraft-webapp:build)
    npm --prefix frontend ci --ignore-scripts
    npm --prefix frontend run build:webapp
    ;;
  artcraft-webapp:deploy)
    npx --yes wrangler@4 deploy --config frontend/apps/artcraft-webapp/wrangler.jsonc
    ;;
  *)
    echo "Usage: ARTCRAFT_APP=<artcraft-website|artcraft-webapp> $0 <build|deploy>" >&2
    echo "Got ARTCRAFT_APP='${APP}', action='${ACTION}'." >&2
    exit 1
    ;;
esac
