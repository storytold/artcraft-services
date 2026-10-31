import { resolvePackageDownloadUrl } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    path: string[];
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { path } = await props.params;
  const filename = path.at(-1);

  if (!filename) {
    return new Response("Not found", { status: 404 });
  }

  const targetUrl = resolvePackageDownloadUrl(filename);
  if (!targetUrl) {
    return new Response("Package not found", { status: 404 });
  }

  return Response.redirect(targetUrl, 302);
}
