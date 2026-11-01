import { getRpmPrimaryGz } from "@/lib/linux-repos";

export async function GET() {
  const gzBuffer = await getRpmPrimaryGz();

  return new Response(new Uint8Array(gzBuffer), {
    headers: {
      "Content-Type": "application/gzip",
      "Cache-Control": "public, max-age=0, must-revalidate",
    },
  });
}
