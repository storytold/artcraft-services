"use client";

import { useSearchParams } from "next/navigation";
import type { ReactNode } from "react";

const SEEDANCE_REF = "sd2fakeyou";

/**
 * Selects between the Seedance promotional header and the default pricing header based on `?ref=`.
 */
export default function PricingHeader({
  defaultHeader,
  seedanceHeader,
}: {
  defaultHeader: ReactNode;
  seedanceHeader: ReactNode;
}) {
  const searchParams = useSearchParams();
  return <>{searchParams.get("ref") === SEEDANCE_REF ? seedanceHeader : defaultHeader}</>;
}
