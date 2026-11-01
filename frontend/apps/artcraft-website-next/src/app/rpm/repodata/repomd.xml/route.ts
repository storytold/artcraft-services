import { getRpmRepomdXml } from "@/lib/linux-repos";

export async function GET() {
  const repomdXml = await getRpmRepomdXml();

  return new Response(repomdXml, {
    headers: {
      "Content-Type": "application/xml; charset=utf-8",
      "Cache-Control": "public, max-age=0, must-revalidate",
    },
  });
}
