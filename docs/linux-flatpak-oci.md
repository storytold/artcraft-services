# Linux Flatpak OCI registry

The website's Flatpak OCI index resolves each listed image tag against GHCR and returns the
registry's real OCI manifest digest. The digest is required by Flatpak to pull the image and must
never be generated from release metadata.

## GHCR package setup

Every image referenced by the index must exist and be public so visitors can install it without
credentials:

- `ghcr.io/storytold/craft-launcher` (the shared release workflow uses the GitHub repository name)
- `ghcr.io/storytold/<app-slug>` for each Craft app included in the website's OCI catalog

The release workflow publishes `<version>-<architecture>` and `stable-<architecture>` tags (for
example, `0.12.0-x86_64` and `stable-x86_64`). After the first push for a package, set its GHCR
package visibility to **Public** in the package settings. A private package can sometimes be read
by the website with a server token, but Flatpak clients still cannot install it anonymously.

## Optional website read token

Public GHCR packages normally allow anonymous index resolution. If the deployment needs an
authenticated read, configure these server-only environment variables on the website deployment:

- `FLATPAK_OCI_READ_USERNAME`: GitHub account that owns the token.
- `FLATPAK_OCI_READ_TOKEN`: a token with `read:packages` access to the images.

Do not use `NEXT_PUBLIC_` names or expose this token to browser code. This token only lets the
website resolve manifest digests; it does not replace public package visibility for end-user
installs. A missing tag or inaccessible image now causes the index request to return an error
instead of publishing a fabricated digest.
