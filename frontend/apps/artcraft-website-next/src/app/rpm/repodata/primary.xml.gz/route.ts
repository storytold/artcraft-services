import { gzipSync } from "node:zlib";
import { generateRpmPrimaryXml } from "@/lib/linux-repos";

export function GET() {
  const xml = generateRpmPrimaryXml();
  const gzBuffer = gzipSync(Buffer.from(xml, "utf-8"));

  return new Response(gzBuffer, {
    headers: {
      "Content-Type": "application/gzip",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
