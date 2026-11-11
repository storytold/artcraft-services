import { generateRpmRepoConfig, getRequestBaseUrl } from "@/lib/linux-repos";

export async function GET(request: Request) {
  const baseUrl = getRequestBaseUrl(request);

  return new Response(await generateRpmRepoConfig(baseUrl), {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
