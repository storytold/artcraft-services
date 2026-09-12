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
│   ├── apps/artcraft-webapp/     # Browser application at app.getartcraft.com
│   ├── apps/artcraft-website/    # Product website at getartcraft.com
│   └── libs/                     # Shared React components and TypeScript libraries
└── Cargo.toml                    # Rust workspace
```

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
