import { getRpmPrimaryGz } from "@/lib/linux-repos";

export function GET() {
  const gzBuffer = getRpmPrimaryGz();

  return new Response(new Uint8Array(gzBuffer), {
    headers: {
      "Content-Type": "application/gzip",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
