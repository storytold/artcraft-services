import { getRpmRepomdXml } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    arch: string;
  }>;
};

const VALID_RPM_ARCHES = new Set(["x86_64", "aarch64", "riscv64", "noarch"]);

export async function GET(_request: Request, props: RouteProps) {
  const { arch } = await props.params;
  if (!VALID_RPM_ARCHES.has(arch)) {
    return new Response(`Architecture "${arch}" not supported`, { status: 404 });
  }

  const repomd = getRpmRepomdXml(arch);

  return new Response(repomd, {
    headers: {
      "Content-Type": "application/xml; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=300",
    },
  });
}
