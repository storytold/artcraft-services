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
  artcraft-dashboard:build)
    # Standalone pnpm app outside the Nx/npm workspace; the script installs and
    # builds inside its own folder.
    bash frontend/apps/artcraft-dashboard/script/build_cloudflare.sh
    ;;
  artcraft-dashboard:deploy)
    npx --yes wrangler@4 deploy --config frontend/apps/artcraft-dashboard/wrangler.jsonc
    ;;
  *)
    echo "Usage: ARTCRAFT_APP=<artcraft-website|artcraft-webapp|artcraft-dashboard> $0 <build|deploy>" >&2
    echo "Got ARTCRAFT_APP='${APP}', action='${ACTION}'." >&2
    exit 1
    ;;
esac
