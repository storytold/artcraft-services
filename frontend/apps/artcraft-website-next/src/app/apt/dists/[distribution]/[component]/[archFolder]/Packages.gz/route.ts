import { handleAptPackagesRoute } from "@/lib/linux-repos";

type RouteProps = {
  params: Promise<{
    distribution: string;
    component: string;
    archFolder: string;
  }>;
};

export async function GET(_request: Request, props: RouteProps) {
  const { distribution, component, archFolder } = await props.params;
  return handleAptPackagesRoute(distribution, component, archFolder, { gzip: true });
}
