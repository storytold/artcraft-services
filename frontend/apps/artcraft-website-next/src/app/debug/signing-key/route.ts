import { describeSigningKey, isRepoSigningConfigured } from "@/lib/repo-signing";

export async function GET() {
  return Response.json(
    { configured: await isRepoSigningConfigured(), key: describeSigningKey() },
    { headers: { "Cache-Control": "no-store" } },
  );
}
