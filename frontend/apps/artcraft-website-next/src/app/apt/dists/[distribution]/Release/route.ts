import { generateAptRelease } from "@/lib/linux-repos";

export function GET() {
  const content = generateAptRelease();

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
