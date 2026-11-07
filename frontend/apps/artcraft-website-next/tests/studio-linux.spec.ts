import { getFlatpakOciAppIds } from "../src/lib/flatpak-oci";
import { expect, test } from "@playwright/test";
import { artcraftStudioDownloads } from "../src/lib/artcraft-studio";
import { generatePacmanDesc, getLinuxRepoApps, resolvePackageDownloadUrl } from "../src/lib/linux-repos";

test("Studio Linux catalog matches every package target", () => {
  expect(getFlatpakOciAppIds()).toContain("ai.artcraft.app");
  const downloads = artcraftStudioDownloads();
  for (const arch of ["x86_64", "aarch64", "riscv64"]) {
    const formats = arch === "riscv64" ? ["deb", "rpm", "tar.gz"]
      : ["deb", "rpm", "tar.gz", "AppImage", "flatpak", "pkg.tar.zst"];
    for (const format of formats) {
      const file = `artcraft-0.41.1-linux-${arch}.${format}`;
      expect(downloads.some((download) => download.fileName === file)).toBeTruthy();
      expect(resolvePackageDownloadUrl(file)).toBe(`https://github.com/storytold/artcraft/releases/download/artcraft-v0.41.1/${file}`);
    }
  }
  expect(resolvePackageDownloadUrl("artcraft-0.41.1-linux-riscv64.flatpak")).toBeNull();
  expect(resolvePackageDownloadUrl("artcraft-0.41.1-linux-x86_64.pkg.tar.zst.sig"))
    .toBe("https://github.com/storytold/artcraft/releases/download/artcraft-v0.41.1/artcraft-0.41.1-linux-x86_64.pkg.tar.zst.sig");
});

test("Studio APT and RPM repositories use the ArtCraft tag and WebKit dependencies", async () => {
  const originalFetch = globalThis.fetch;
  const requests: string[] = [];
  const files = ["x86_64", "aarch64", "riscv64"].flatMap((arch) =>
    ["deb", "rpm", "rpm.asc", "pkg.tar.zst", "pkg.tar.zst.sig"].map((format) => `artcraft-0.41.1-linux-${arch}.${format}`));
  globalThis.fetch = async (input) => {
    const url = input instanceof Request ? input.url : String(input);
    requests.push(url);
    if (url.endsWith("SHA256SUMS.txt")) {
      return new Response(url.includes("/artcraft/releases/")
        ? files.map((file) => `${"ab".repeat(32)}  ${file}`).join("\n") : "");
    }
    return new Response(null, { headers: { "Content-Length": "4096" } });
  };
  try {
    const apps = await getLinuxRepoApps({ deb: true, rpm: true, arch: true });
    const studio = apps.find((app) => app.slug === "artcraft");
    expect(studio).toBeDefined();
    expect(studio!.debAssets).toHaveLength(3);
    expect(studio!.debAssets.every((asset) => asset.depends.includes("libwebkit2gtk-4.1-0"))).toBeTruthy();
    expect(studio!.debAssets.find((asset) => asset.arch === "riscv64")!.depends).toContain("2.39");
    expect(studio!.archAssets).toHaveLength(2);
    const pacmanDesc = generatePacmanDesc(studio!, studio!.archAssets[0]);
    expect(pacmanDesc).toContain("%NAME%\nai.artcraft.app\n");
    expect(pacmanDesc).toContain("%BASE%\nai.artcraft.app\n");
    expect(requests).toContain("https://github.com/storytold/artcraft/releases/download/artcraft-v0.41.1/SHA256SUMS.txt");
  } finally {
    globalThis.fetch = originalFetch;
  }
});
