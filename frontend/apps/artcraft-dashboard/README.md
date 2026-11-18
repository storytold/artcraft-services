# ArtCraft Admin Dashboard

Admin dashboard for managing ArtCraft, served at
<https://dashboard.artcraft.team>.

## Deployment

Production is a Cloudflare Worker with static assets
([`wrangler.jsonc`](./wrangler.jsonc)). Deploy from the repository root:

```bash
npm run build:dashboard
npm run deploy:dashboard
```

Workers Builds settings for the `artcraft-dashboard` Worker:

| Setting        | Value                      |
|----------------|----------------------------|
| Build command  | `npm run build:dashboard`  |
| Deploy command | `npm run deploy:dashboard` |
| Build variable | `NODE_VERSION=22`          |

`npm run build:dashboard` and `npm run deploy:dashboard` set
`ARTCRAFT_APP=artcraft-dashboard` inline and pass this app's `wrangler.jsonc` to
Wrangler, so the Worker needs no build variables beyond `NODE_VERSION`. Vite 7
needs Node >= 20.19 / 22.12, so `NODE_VERSION=22` is required; it cannot be
pinned in the repository because the rest of the workspace builds on the same
Node version.

### Build secrets

This app needs none. `LAUNCHER_CATALOG_SIGNING_KEY` and `LINUX_SIGNING_KEY`
belong to the website's static-export pipeline
([`sign-launcher-catalog.mjs`](../artcraft-website-next/scripts/sign-launcher-catalog.mjs)),
which only `artcraft-website-next` runs. Nothing under this app reads them.

Do not configure this Worker with plain `npm run build` / `npm run deploy`. Those
default to `ARTCRAFT_APP=artcraft-website`, so the build runs the website's
signing step (failing without `LAUNCHER_CATALOG_SIGNING_KEY`) and the deploy hands
Wrangler the root config, failing with
`The directory specified by the "assets.directory" field ... does not exist:`
`.../frontend/apps/artcraft-website-next/out`.

### `/api` proxy

[`cloudflare_worker/index.ts`](./cloudflare_worker/index.ts) forwards `/api/*` to
`https://api.storyteller.ai`. The dashboard calls the API with
`credentials: "include"`, so the session cookie has to be first-party; a direct
cross-site call would be dropped by browsers. The Worker forwards the browser's
`Origin`, so `https://dashboard.artcraft.team` must stay in the API's CORS
allowlist
([`artcraft_admin_dashboard.rs`](../../../crates/lib/actix_cors_configs/src/configs/artcraft_admin_dashboard.rs)).
The Worker's `*.workers.dev` host is not on that allowlist, so API calls only
work through `dashboard.artcraft.team` — add the test host to the CORS config if
you need to exercise a deployment before attaching the domain.

### Netlify rollback

[`netlify.toml`](./netlify.toml) and [`script/netlify_build.sh`](./script/netlify_build.sh)
are kept so the previous site can still build and serve. Netlify's routing rules
live in [`_redirects`](./_redirects) at the app root and are copied into `dist/`
by that script; Cloudflare does not get them, because Workers reject the
`/* /index.html 200` fallback and cannot proxy external hosts from `_redirects`.
