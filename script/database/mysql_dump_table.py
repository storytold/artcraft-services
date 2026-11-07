#!/usr/bin/env python3
"""
Back up a single production database table to a local SQL file using mysqldump.

Reads connection info from secrets.toml (same format as the other database scripts).

Usage:
  python mysql_dump_table.py --table=users
  python mysql_dump_table.py --table=media_files --directory=/tmp/backups

Requirements:
  pip install pymysql
  mysqldump must be on PATH
"""

import argparse
import os
import shutil
import subprocess
import sys
import tomllib
from datetime import datetime
from pathlib import Path
from urllib.parse import urlparse

try:
    import pymysql
except ImportError:
    print("pymysql is required: pip install pymysql")
    sys.exit(1)


SECRETS_PATH = Path(__file__).parent / "secrets.toml"

DEFAULT_BACKUP_DIR = Path.home() / "database" / "backups"

ALLOWED_TABLES = [
    "audit_logs",
    "batch_generations",
    "favorites",
    "featured_items",
    "generic_inference_jobs",
    "google_sign_in_accounts",
    "media_files",
    "model_weights",
    "prompt_context_items",
    "prompts",
    "tag_uses",
    "tags",
    "user_bookmarks",
    "user_ratings",
    "user_roles",
    "user_sessions",
    "user_stripe_customer_links",
    "user_subscriptions",
    "users",
    "wallet_ledger_entries",
    "wallets",
]


def load_config():
    if not SECRETS_PATH.exists():
        print(f"Error: secrets.toml not found at {SECRETS_PATH}")
        print(f"Copy secrets.toml.example to secrets.toml and fill in the connection URL.")
        sys.exit(1)

    with open(SECRETS_PATH, "rb") as f:
        config = tomllib.load(f)
    url = config["database"]["url"]
    parsed = urlparse(url)
    return {
        "host": parsed.hostname,
        "port": parsed.port or 3306,
        "user": parsed.username,
        "password": parsed.password,
        "database": parsed.path.lstrip("/"),
    }


def connect(config):
    return pymysql.connect(
        host=config["host"],
        port=config["port"],
        user=config["user"],
        password=config["password"],
        database=config["database"],
        charset="utf8mb4",
        cursorclass=pymysql.cursors.DictCursor,
    )


def table_exists(conn, table_name):
    with conn.cursor() as cursor:
        cursor.execute("SHOW TABLES LIKE %s", (table_name,))
        return cursor.fetchone() is not None


def count_rows(conn, table_name):
    with conn.cursor() as cursor:
        # table_name is checked against ALLOWED_TABLES in main() before this is reached.
        cursor.execute(f"SELECT COUNT(*) AS cnt FROM `{table_name}`")  # NOSONAR
        result = cursor.fetchone()
        return result["cnt"]


def main():
    parser = argparse.ArgumentParser(description="Back up a database table using mysqldump.")
    parser.add_argument("--table", required=True, help="Name of the table to back up.")
    parser.add_argument("--directory", default=None, help=f"Directory to save the dump. Default: {DEFAULT_BACKUP_DIR}")
    args = parser.parse_args()

    table_name = args.table
    backup_dir = Path(args.directory) if args.directory else DEFAULT_BACKUP_DIR

    # Validate table is in allowed list
    if table_name not in ALLOWED_TABLES:
        print(f"Error: '{table_name}' is not in the allowed tables list.")
        print(f"Allowed tables: {', '.join(sorted(ALLOWED_TABLES))}")
        sys.exit(1)

    # Check mysqldump is available
    if shutil.which("mysqldump") is None:
        print("Error: mysqldump not found on PATH.")
        sys.exit(1)

    config = load_config()

    # Verify table exists and count rows
    conn = connect(config)
    try:
        if not table_exists(conn, table_name):
            print(f"Error: table '{table_name}' does not exist in database '{config['database']}'.")
            sys.exit(1)

        row_count = count_rows(conn, table_name)
    finally:
        conn.close()

    # Build output path
    # The operator picks the output directory with --directory; this is a local backup tool.
    backup_dir.mkdir(parents=True, exist_ok=True)  # NOSONAR
    timestamp = datetime.now().strftime("%Y-%m-%d-%H-%M-%S")
    output_file = backup_dir / f"{table_name}_{timestamp}.sql"

    # Confirm with user
    print(f"Table:     {table_name}")
    print(f"Database:  {config['database']} @ {config['host']}:{config['port']}")
    print(f"Rows:      {row_count:,}")
    print(f"Output:    {output_file}")
    print()

    confirm = input("Proceed with backup? [y/N] ").strip().lower()
    if confirm not in ("y", "yes"):
        print("Aborted.")
        sys.exit(0)

    # Run mysqldump
    cmd = [
        "mysqldump",
        f"--host={config['host']}",
        f"--port={config['port']}",
        f"--user={config['user']}",
        f"--password={config['password']}",
        "--single-transaction",
        "--routines",
        "--triggers",
        "--set-gtid-purged=OFF",
        config["database"],
        table_name,
    ]

    print(f"\nRunning mysqldump for '{table_name}'...")

    # output_file lives under the operator-chosen backup directory.
    with open(output_file, "w") as f:  # NOSONAR
        # argv, no shell; table_name is allowlisted and the rest comes from secrets.toml.
        result = subprocess.run(cmd, stdout=f, stderr=subprocess.PIPE, text=True)  # NOSONAR

    if result.returncode != 0:
        print(f"Error: mysqldump failed (exit code {result.returncode})")
        if result.stderr:
            # Filter out the password warning which is expected
            for line in result.stderr.strip().splitlines():
                if "Using a password on the command line" not in line:
                    print(f"  {line}")
        sys.exit(1)

    file_size = output_file.stat().st_size
    if file_size < 100:
        print(f"Warning: output file is suspiciously small ({file_size} bytes).")

    print(f"Done. Backup saved to: {output_file} ({file_size:,} bytes)")


if __name__ == "__main__":
    main()
