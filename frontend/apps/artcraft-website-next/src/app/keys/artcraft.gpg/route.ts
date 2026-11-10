import { getSigningPublicKey } from "@/lib/repo-signing";

export async function GET() {
  const key = await getSigningPublicKey();
  if (!key) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  return new Response(new Uint8Array(key.binary), {
    headers: {
      "Content-Type": "application/pgp-keys",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
