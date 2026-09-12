# Development setup

This repository contains the ArtCraft backend and web frontends. Desktop development
lives in [storytold/artcraft](https://github.com/storytold/artcraft).

## Backend

Install Rust and Cargo, then follow the [server setup guide](./dev_setup_server.md)
for MySQL and Redis. The server also needs Elasticsearch configuration, object
storage, and credentials for the integrations you use. Configuration files live in
[`storyteller_web/config`](../crates/service/web/storyteller_web/config); dependency
initialization lives in [`startup`](../crates/service/web/storyteller_web/src/startup).

From the repository root, with those dependencies configured:

```bash
SQLX_OFFLINE=true cargo check -p storyteller-web
SQLX_OFFLINE=true cargo run -p storyteller-web
```

The API listens on `http://localhost:12345` by default. `SQLX_OFFLINE=true` uses
checked-in query metadata at build time; the server still needs MySQL at runtime.
See the [root README](../README.md) for configuration loading and worker setup.

## Web frontends

Install Node.js and npm, then run from the repository root:

```bash
cd frontend
npm install
VITE_USE_LOCAL_API=true npx nx dev artcraft-webapp
```

The web app runs at `http://localhost:4201`, using the local backend. To develop
against the hosted API, omit `VITE_USE_LOCAL_API=true`.

For the product website, run `npx nx dev artcraft-website` from `frontend`; it uses
`http://localhost:4200`. Repository-root launchers live under [`script/website`](../script/website).
See the [frontend README](../frontend/README.md) for builds and troubleshooting.
