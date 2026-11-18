#!/bin/bash
#
# Regenerate the committed OpenAPI document served at storyteller-docs.artcraft.team.
#
#   bash script/build_api_docs.sh
#
# The document is produced by the `docs-cli` binary (utoipa), which builds the
# whole workspace. Cloudflare Workers Builds cannot run that, so
# frontend/api_docs/public/api.json is committed and this script refreshes it.
# Run it after changing an endpoint or a request/response type, then commit the
# regenerated file with the change.

set -euo pipefail

cd "$(dirname "$0")/.."

export SQLX_OFFLINE=true

# boring-sys2 uses bindgen (libclang) to generate BoringSSL FFI bindings. On
# machines where clang cannot find stddef.h on its own, point it at GCC's own
# include directory instead of installing packages.
if command -v gcc >/dev/null 2>&1; then
  export BINDGEN_EXTRA_CLANG_ARGS="-I$(gcc -print-file-name=include)"
fi

cargo run --bin docs-cli

# docs-cli writes api.json into the working directory, which is the repo root.
mv api.json frontend/api_docs/public/api.json

echo "Wrote frontend/api_docs/public/api.json ($(wc -c < frontend/api_docs/public/api.json) bytes)"
