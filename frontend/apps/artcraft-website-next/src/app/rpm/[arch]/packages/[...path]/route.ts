import { resolvePackageDownloadUrl } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    arch: string;
    path: string[];
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { path } = await props.params;
  const filename = path[path.length - 1];

  const targetUrl = resolvePackageDownloadUrl(filename);
  if (!targetUrl) {
    return new Response(`Package asset "${filename}" not found in release catalog`, {
      status: 404,
      headers: { "Content-Type": "text/plain; charset=utf-8" },
    });
  }

  return Response.redirect(targetUrl, 302);
}
