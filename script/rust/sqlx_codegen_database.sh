#!/bin/bash
#
# Regenerates the workspace-root `.sqlx/` offline query cache for
# `mysql_queries`, checked against the local development MySQL database.
# Uses DATABASE_URL from the environment or the repository-root `.env`.
#
# Requirements:
#   - sqlx-cli matching the workspace sqlx version, with the MySQL driver:
#       cargo install sqlx-cli --version 0.7.4 --no-default-features \
#         --features mysql,rustls --locked
#   - A running local MySQL with the migrated dev database.
#
# The new cache is staged in a temp directory and only replaces the old
# `.sqlx/*.json` files after preparation succeeds, so a failure part-way
# never leaves the repo without a query cache.

set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

mysql_package_path="${root_dir}/crates/schema/database/mysql_queries"

query_cache_dir="${root_dir}/.sqlx"
staging_dir="$(mktemp -d /tmp/sqlx_codegen.XXXXXX)"

# The dev MySQL database. Falls back to the DATABASE_URL in the repo root
# .env so the script works from any directory.
mysql_database_url="${DATABASE_URL:-$(grep -m1 '^DATABASE_URL=mysql' "${root_dir}/.env" | cut -d= -f2-)}"

prepare_mysql() {
  echo "Preparing the MySQL query cache..."
  pushd "${mysql_package_path}"
  cargo sqlx prepare \
    --database-url "${mysql_database_url}"
  popd

  mv "${mysql_package_path}/.sqlx/"*.json "${staging_dir}/"
}

replace_query_cache() {
  echo "Replacing the query cache..."
  mkdir -p "${query_cache_dir}"
  rm -f "${query_cache_dir}/"*.json
  mv "${staging_dir}/"*.json "${query_cache_dir}/"
  rmdir "${staging_dir}"

  # Remove the (now empty) per-package cache directory so sqlx never resolves
  # offline queries against a stale crate-local cache instead of the
  # workspace root one.
  rmdir "${mysql_package_path}/.sqlx" 2>/dev/null || true
}

# The sqlx macros only emit query metadata when the crates actually
# recompile; a fresh (cached) build would yield an EMPTY cache. Force the
# query crate to rebuild.
cargo clean -p mysql_queries

# Prepare must expand the macros against the live databases.
export SQLX_OFFLINE=false

prepare_mysql
replace_query_cache

echo 'done'
