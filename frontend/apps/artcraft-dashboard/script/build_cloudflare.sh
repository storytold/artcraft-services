#!/bin/bash

set -euxo pipefail

# Build the ArtCraft admin dashboard for Cloudflare Workers.
#
# Run from anywhere: this builds inside the app's own folder, which is deliberate
# (see netlify_build.sh) — the dashboard is a standalone pnpm app with React 19 +
# Vite 7, outside the Nx/npm workspace.

cd "$(dirname "$0")/.."

echo "Building artcraft-dashboard (standalone pnpm build)"

# Use the pnpm version pinned in package.json's `packageManager` field, via
# corepack. Disable the interactive download prompt so CI never blocks waiting
# for confirmation.
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0

corepack pnpm install --frozen-lockfile
corepack pnpm build

echo "Final build files:"
find dist/
