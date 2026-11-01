import rawPackageData from "./linux-package-data.json";

/**
 * Cryptographic hashes and byte sizes for published Linux package assets (.deb, .rpm, and .pkg.tar.zst).
 * Automatically synchronized against official storytold release assets and SHA256SUMS.txt.
 * DO NOT EDIT MANUALLY - run `npm run sync:packages` to update.
 */
export type LinuxPackageAssetMeta = {
  size: number;
  sha256: string;
  installedSize: number;
};

export const LINUX_PACKAGE_METADATA: Record<string, LinuxPackageAssetMeta> =
  rawPackageData as Record<string, LinuxPackageAssetMeta>;
