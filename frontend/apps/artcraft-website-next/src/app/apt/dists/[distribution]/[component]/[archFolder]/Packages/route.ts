import {
  generateAptPackages,
  VALID_APT_DISTRIBUTIONS,
  VALID_APT_COMPONENTS,
  VALID_APT_ARCHES,
} from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    distribution: string;
    component: string;
    archFolder: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { distribution, component, archFolder } = await props.params;

  if (
    !VALID_APT_DISTRIBUTIONS.has(distribution) ||
    !VALID_APT_COMPONENTS.has(component) ||
    !archFolder.startsWith("binary-") ||
    !VALID_APT_ARCHES.has(archFolder.slice(7))
  ) {
    return new Response("Not found", { status: 404 });
  }

  const arch = archFolder.slice(7);
  const content = generateAptPackages(arch);

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=60, s-maxage=60",
    },
  });
}
