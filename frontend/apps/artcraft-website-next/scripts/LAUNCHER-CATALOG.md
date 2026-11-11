# Hosted launcher catalog

`public/launcher-catalog.toml` preserves the PR's `[[apps]]` schema and display
order. The generated data comes from the launcher's bundled `catalog.toml` after
launcher PR #1115, or its compiled `catalog.rs`, `details.rs`, and
`desktop_entry.rs` on older revisions, including aliases, publisher and macOS
signing metadata, app descriptions, features, screenshots, and desktop integration.
An absent `mac_team` means the launcher must not install that app on macOS.
`mac_bundle_name` is the primary macOS `.app` stem, independent of display name.

## Refresh and compare

Requires Python 3.12+ (stdlib only), Git, and rustc. From this app directory:

```sh
git -C /path/to/craft-launcher fetch origin main
python3 scripts/sync-launcher-catalog.py /path/to/craft-launcher --revision origin/main
python3 scripts/sync-launcher-catalog.py /path/to/craft-launcher --revision origin/main --check
```

The command reads a committed revision, ignoring uncommitted launcher changes.
The catalog records the full source commit. `--check` compares every exported
field and the icon bytes, hashes, licenses and attribution without writing files.
Review changes to repositories and signing teams before publishing. The command
copies icons without modification and leaves older hashed icons available for
clients still using an earlier catalog.

## Cloudflare build signing

The existing `npm run build:cloudflare` static-export pipeline now signs the final
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
