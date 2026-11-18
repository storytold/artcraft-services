# API docs — storyteller-docs.artcraft.team

Swagger UI for the `storyteller-web` HTTP API. Production:
<https://storyteller-docs.artcraft.team>.

This is a static-assets Cloudflare Worker with no server-side code:

| Path                  | What                                                                      |
|-----------------------|---------------------------------------------------------------------------|
| `public/index.html`   | Swagger UI page; loads `api.json` from its own directory                  |
| `public/api.json`     | OpenAPI 3.1 document generated from the Rust sources                      |
| `wrangler.jsonc`      | Worker `storyteller-docs`, custom domain `storyteller-docs.artcraft.team` |

## Why the spec is committed

`public/api.json` is produced by `cargo run --bin docs-cli`
(`crates/service/web/storyteller_web/src/api_doc_cli.rs`), which builds the whole
workspace. Cloudflare Workers Builds has no Rust toolchain and that build would be
far too slow for a deploy, so the generated document is committed and the Worker
has no build step of its own: the build command validates the artifacts and the
deploy command uploads them.

Refresh it whenever the HTTP API changes:

```bash
bash script/build_api_docs.sh
```

Then commit `public/api.json` alongside the endpoint change, so the diff shows the
API surface that moved.

## Deploying

```bash
npm run build:docs    # validates the artifacts
npm run deploy:docs   # npx wrangler deploy --config frontend/api_docs/wrangler.jsonc
```

Workers Builds runs both from the repository root. `storyteller-docs.artcraft.team` is a
`custom_domain` route, so Cloudflare creates the DNS record and certificate; the
`artcraft.team` zone must be on the same account as the Worker (the setup
`dashboard.artcraft.team` already uses).

These docs previously lived at `storyteller-docs.netlify.app`. That Netlify site is
no longer maintained, and the Worker deliberately reuses its name.
