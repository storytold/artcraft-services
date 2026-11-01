import { generatePacmanDb, VALID_PACMAN_ARCHES } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    arch: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { arch } = await props.params;
  if (!VALID_PACMAN_ARCHES.has(arch)) {
    return new Response(`Architecture "${arch}" not supported`, { status: 404 });
  }

  const dbBuffer = await generatePacmanDb(arch);

  return new Response(new Uint8Array(dbBuffer), {
    headers: {
      "Content-Type": "application/gzip",
      "Cache-Control": "public, max-age=0, must-revalidate",
    },
  });
}
