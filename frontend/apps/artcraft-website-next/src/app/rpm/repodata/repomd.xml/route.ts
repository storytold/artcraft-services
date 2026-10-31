import { gzipSync } from "node:zlib";
import { generateRpmPrimaryXml, generateRpmRepomdXml } from "@/lib/linux-repos";

export function GET() {
  const xml = generateRpmPrimaryXml();
  const xmlBuffer = Buffer.from(xml, "utf-8");
  const gzBuffer = gzipSync(xmlBuffer);

  const repomdXml = generateRpmRepomdXml(xmlBuffer, gzBuffer);

  return new Response(repomdXml, {
    headers: {
      "Content-Type": "application/xml; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
