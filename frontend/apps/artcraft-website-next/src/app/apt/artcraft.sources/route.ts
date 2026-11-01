import { generateAptSourcesDeb822, getRequestBaseUrl } from "@/lib/linux-repos";

export function GET(request: Request) {
  const baseUrl = getRequestBaseUrl(request);

  return new Response(generateAptSourcesDeb822(baseUrl), {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
