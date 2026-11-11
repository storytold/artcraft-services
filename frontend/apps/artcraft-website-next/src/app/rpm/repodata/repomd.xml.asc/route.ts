import { getRpmRepomdXml } from "@/lib/linux-repos";
import { isRepoSigningConfigured, signDetachedArmored } from "@/lib/repo-signing";

export async function GET() {
  if (!(await isRepoSigningConfigured())) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  const signature = await signDetachedArmored(await getRpmRepomdXml());
  if (!signature) {
    return new Response("Repository signing is not configured", { status: 404 });
  }

  return new Response(signature, {
    headers: {
      "Content-Type": "application/pgp-signature",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
