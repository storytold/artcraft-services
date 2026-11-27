# Hosted launcher catalog

The canonical base catalog, content-addressed app icons, licenses, attribution and
versioned screenshots live in [storytold/craft-launcher-assets](https://github.com/storytold/craft-launcher-assets).
This repository does not commit duplicate copies. The Cloudflare production build fetches
one committed main revision, records its SHA in the generated catalog, and copies the
published screenshot tags into their existing versioned URLs. The build reads Git blobs
only, not code from the assets repository. A missing or inconsistent asset fails the new
deployment; the previous successful static deployment remains available.

Keep all published hashed icons and screenshot tags immutable. A main push or a new
screenshot version tag sends a `launcher-assets-changed` dispatch using the assets repo's
`ARTCRAFT_SERVICES_DISPATCH_TOKEN`. The services release reconciliation workflow handles
that event through the existing Cloudflare deploy hook. Its hourly reconciliation remains
a fallback. For local development, run `npm run sync:launcher-assets` from this app directory
to populate ignored `public/` asset files; no signing key is needed.

## Cloudflare build generation and signing

The existing `npm run build:cloudflare` static-export pipeline enriches
the fetched base catalog in `out/launcher-catalog.toml` via
`scripts/generate-launcher-catalog.mjs` before signing. Similar to the static
Linux repositories (`scripts/generate-static-repos.mjs`), it reads the latest
stable, prerelease, and nightly versions from `src/lib/crafting-app-releases.ts`
(kept fresh hourly by `.github/workflows/update-craft-release-versions.yml` and
on release-change dispatch; configure `CLOUDFLARE_DEPLOY_HOOK` to rebuild even
when the recorded version strings have not changed),
fetches each release's `SHA256SUMS.txt` from GitHub, and writes:

- top-level `generated_at` (RFC 3339 UTC timestamp),
- top-level `[launcher]` with `[launcher.releases.<channel>]` and
  `[[launcher.releases.<channel>.assets]]` for `artcraft-launcher`, and
- per-app `[apps.releases.<channel>]` and `[[apps.releases.<channel>.assets]]`
  (`stable`, `prerelease`, `nightly`) inside each `[[apps]]` entry.

`scripts/sign-launcher-catalog.mjs` then signs the final
`out/launcher-catalog.toml` bytes. It writes `out/launcher-catalog.toml.sig` as a
base64-encoded, 64-byte Ed25519 detached signature followed by a newline. This
format must be matched by the launcher verifier; it is not a minisign envelope.
Missing, invalid, or non-Ed25519 keys fail the build. Local `npm run dev` and
`npm run build` do not require a secret.

Generate the key once on a trusted machine (do not commit the private key):

```sh
openssl genpkey -algorithm ED25519 -out launcher-catalog-private.pem
openssl pkey -in launcher-catalog-private.pem -pubout -out launcher-catalog-public.pem
openssl base64 -A -in launcher-catalog-private.pem
```

Set **`LAUNCHER_CATALOG_SIGNING_KEY`** to the last command's output as an encrypted
**build environment** secret in Cloudflare. It is the base64-encoded PKCS#8 PEM
private key, not a raw seed or a public key. Configure it in each deployment
context that runs this build; untrusted fork previews should not receive it.
Do not prefix it with `NEXT_PUBLIC_` or place it in runtime Wrangler bindings.
Keep the public key for pinning in the launcher. No production key is generated
or committed by this PR.

Cloudflare Pages should use this app as the root directory,
`npm run build:cloudflare` as the build command and `out` as the asset output.
The repository's existing Wrangler Worker deployment uses the same `out` assets
and build pipeline. `public/_headers` supplies CORS, catalog/signature revalidation,
and immutable caching for hashed icons in both Cloudflare static asset modes.
Next dev/server mode mirrors those headers; static exports omit Next headers.
This change does not migrate the existing site's dynamic Worker routes to Pages.
See [Cloudflare header rules](https://developers.cloudflare.com/pages/configuration/headers/)
and [build configuration](https://developers.cloudflare.com/pages/configuration/build-configuration/).

The launcher must decode the signature, verify the exact catalog bytes against
its compiled-in public key, and enforce its approved repository/signing policy
before using hosted data. Serving the public key beside the file would not
establish trust. Remote fetch, signature verification, cache/fallback and UI
consumption remain launcher-side work; this PR hosts the compatible data only.

Run signing checks with `npm run test:launcher-catalog`. They use ephemeral keys,
verify exact bytes, and reject tampering, the wrong key, and invalid configuration.

## Announcements

`public/launcher-announcements.toml` holds the launcher's announcements and alerts. The same
build step signs it with the same key, writing `out/launcher-announcements.toml.sig`, and
`public/_headers` serves both like the catalog. The launcher shows notices only from this signed
file. Its format and limits are in craft-launcher's `docs/announcements.md`. A file with no rows
is valid and shows nothing. Give anything time-limited an `expires`: the signature proves who
wrote the file, not when.

To withdraw notices, remove their rows and deploy. Do not delete the file: launchers that already
fetched it keep their last verified copy while the file is missing or its signature fails.
