import { getRpmRepomdXml } from "@/lib/linux-repos";

export function GET() {
  const repomdXml = getRpmRepomdXml();

  return new Response(repomdXml, {
    headers: {
      "Content-Type": "application/xml; charset=utf-8",
      "Cache-Control": "public, max-age=60, s-maxage=60",
    },
  });
}
