import { handlePackageDownloadRedirect } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    arch: string;
    path: string[];
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { path } = await props.params;
  return handlePackageDownloadRedirect(path);
}
