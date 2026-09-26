# AGENTS.md

This file provides guidance for coding agents when working with the Artcraft monorepo.

## Project Overview

ArtCraft Services contains the Rust backend, HTTP API, workers, and TypeScript web
frontends for ArtCraft. The Tauri desktop application is maintained separately at
https://github.com/storytold/artcraft.

## Project Structure

```text
artcraft-services/
├── _database/                    # SQL migrations and Elasticsearch schemas
│   └── sql/
│       ├── migrations/           # Server MySQL migrations
│       └── migrations_squashed/  # Materialized MySQL table definitions
├── _tools/postman/               # HTTP request collections
├── build/                        # Service Dockerfiles
├── crates/
│   ├── api_clients/              # ArtCraft API types, clients, router, provider clients
│   ├── cli/                      # Development and operations tools
│   ├── lib/                      # Shared Rust utilities
│   ├── schema/                   # Database queries, public tokens/enums, bucket paths
│   │   └── database/
│   │       ├── mysql_queries/    # SQLx queries shared by services, jobs, and CLI tools
│   │       ├── redis_common/     # Redis support
│   │       └── redis_schema/     # Redis key and HKEY topology
│   └── service/
│       ├── job/                  # Provider workers, media processing, analytics
│       ├── plugins/              # Shared billing and service components
│       └── web/storyteller_web/  # Main Actix Web HTTP API
├── frontend/
│   ├── apps/artcraft-webapp/       # Browser application at app.getartcraft.com
│   ├── apps/artcraft-website-next/ # Current public website at getartcraft.com (Next.js)
│   ├── apps/artcraft-website/      # Legacy product website (Vite)
│   └── libs/                      # Shared React components and TypeScript libraries
└── Cargo.toml                    # Rust workspace
```

## Updating Desktop Download Links

Desktop release links are configured independently in **three places**. When
updating the advertised ArtCraft release, update both `WINDOWS_VERSION` and
`MAC_VERSION` in all three unless the request explicitly targets one platform or
app:

1. **Current public website (Next.js):**
   [frontend/apps/artcraft-website-next/src/lib/download-links.ts](frontend/apps/artcraft-website-next/src/lib/download-links.ts).
   This controls the installer links and visible version at
   `https://getartcraft.com/download`. Add the release to `DOWNLOAD_HISTORY`
   (newest first), then update both version selectors.
2. **Legacy website (Vite):**
   [frontend/apps/artcraft-website/src/config/github_download_links.ts](frontend/apps/artcraft-website/src/config/github_download_links.ts).
   Add the release to `DOWNLOAD_HISTORY` and update both version selectors.
   Updating only this file does **not** update the current public website.
3. **Browser webapp:**
   [frontend/apps/artcraft-webapp/src/config/github_download_links.ts](frontend/apps/artcraft-webapp/src/config/github_download_links.ts).
   Update both version constants; these links are used by the welcome and
   checkout-success pages at `app.getartcraft.com`.

Keep previous releases for rollback. The Next.js download page reads its visible
version from `DOWNLOAD_VERSIONS`, which uses the same selectors as the URLs; do
not hardcode a separate version in the page component.

Check that all three configs produce the requested release tag and installer
filenames for Windows (`ArtCraft_<version>_x64-setup.exe`) and macOS
(`ArtCraft_<version>_universal.dmg`). Run `npm run typecheck` from
`frontend/apps/artcraft-website-next` after changing its config. Repository edits
require deployment of the affected apps before the public pages change; do not
report a local update as a deployed update.

## Code Style

- Rust with no minimum supported version
- Actix-web for HTTP services
- SQLx for MySQL and SQLite; prefer `sqlx::query!` / `sqlx::query_as!` compile-time checked macros over runtime `sqlx::query()` whenever possible
- MySQL queries belong in the `mysql_queries` crate, not in application/service crates like `storyteller-web` — call those query functions instead of embedding `sqlx::query!` / `sqlx::query()` in handlers. (Query-writing conventions live in `crates/schema/database/mysql_queries/AGENTS.md`.)
- A mix of wreq and reqwest for Rust HTTP clients
- Never use `println!` or `eprintln!` outside of tests; use `log` crate macros instead
- When two crates export the same type name, alias imports with a suffix: `use foo::Bar as BarFoo;`
- Prefer `use` imports over inline fully-qualified paths; only qualify inline for true one-offs or std collisions
- TypeScript with Nx, React, Vite, Zustand, and Three.js
- Use two spaces for indentation

### File Layout

Organize for top-to-bottom reading. Important things first, details later.

- **Constants** at the top (after imports)
- **Structs/enums** next; outer structs above inner sub-structs
- **API types** in order: Request, Response, Error
- **In impl blocks**: constructors first, then public methods, then private helpers
- Private helpers go *below* the methods that call them
- Among helpers: meatier logic above leaf-level formatters
- **In test modules**: constants first, then test cases (grouped into sub-modules when 2+), then helper functions last

## Markdown

- **Tables must be space-padded so columns align in plain text.** Markdown
  tables are read raw (terminals, diffs, editors) at least as often as
  rendered, and condensed tables are unreadable there. Pad every cell to its
  column width:

  ```markdown
  | Model        | Configuration | Credits    | Speed | Score   |
  |--------------|---------------|--------------------|---------|
  | Meshy 6      | text or image | 104        | 80.0  | +24     |
  | Rodin 2.5    | text or image | 13         | 10.0  | +3      |
  ```

  Not: `| Model | Configuration | Credits | Speed | Score |` packed tight
  with varying widths per row.
