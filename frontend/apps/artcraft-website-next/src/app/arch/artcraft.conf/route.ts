import { generatePacmanRepoConfig, getRequestBaseUrl } from "@/lib/linux-repos";

export async function GET(request: Request) {
  const baseUrl = getRequestBaseUrl(request);
  const content = generatePacmanRepoConfig(baseUrl);

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=0, must-revalidate",
    },
  });
}
