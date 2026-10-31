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

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
