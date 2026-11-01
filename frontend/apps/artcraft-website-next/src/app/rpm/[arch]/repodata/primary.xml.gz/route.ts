import { getRpmPrimaryGz, VALID_RPM_ARCHES } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    arch: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { arch } = await props.params;
  if (!VALID_RPM_ARCHES.has(arch)) {
    return new Response(`Architecture "${arch}" not supported`, { status: 404 });
  }

  const primaryGzBuffer = getRpmPrimaryGz(arch);

  return new Response(new Uint8Array(primaryGzBuffer), {
    headers: {
      "Content-Type": "application/gzip",
      "Cache-Control": "public, max-age=0, must-revalidate",
    },
  });
}
