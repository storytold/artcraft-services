import { gzipSync } from "node:zlib";
import { generateAptPackages } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    distribution: string;
    component: string;
    archFolder: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { archFolder } = await props.params;
  const arch = archFolder.replace(/^binary-/, "");

  const content = generateAptPackages(arch);
  const gzBuffer = gzipSync(Buffer.from(content, "utf-8"));

  return new Response(gzBuffer, {
    headers: {
      "Content-Type": "application/gzip",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
