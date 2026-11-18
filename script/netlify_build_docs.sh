#!/bin/bash
#
# Netlify build for https://storyteller-docs.netlify.app, the previous home of
# the API docs. Production now serves storyteller-docs.artcraft.team from the Cloudflare
# Worker in frontend/api_docs, and this script exists only as the rollback path.
# The Netlify site is UI-configured (there is no netlify.toml for it):
#   * Base directory:    (repository root)
#   * Build command:     bash script/netlify_build_docs.sh
#   * Publish directory: build
#
set -euxo pipefail

# Resolve the repo root regardless of where this script is invoked from
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# Install Rust toolchain
rustup update
rustup default stable

# Emit versions
rustup show

mkdir -p ./build

# Regenerates frontend/api_docs/public/api.json (includes the bindgen workaround).
bash script/build_api_docs.sh

cp ./frontend/api_docs/public/index.html ./build/index.html
cp ./frontend/api_docs/public/api.json ./build/api.json


