import { getSigningPublicKey } from "@/lib/repo-signing";

/**
 * libzypp looks for the metadata signing key next to the index (`repodata/repomd.xml.key`, the
 * openSUSE Build Service convention) and reports that URL in `zypper lr -d`. Serving it lets zypper
 * fetch and import the key that signs repomd.xml on its own.
 */
export async function GET() {
  const key = await getSigningPublicKey();
  if (!key) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  return new Response(key.armored, {
    headers: {
      "Content-Type": "application/pgp-keys; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
