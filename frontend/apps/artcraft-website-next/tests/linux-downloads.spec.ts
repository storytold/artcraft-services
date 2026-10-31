import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";

const cases = [
  { arch: "x86_64", userAgentArch: "x86_64", extension: "flatpak" },
  { arch: "aarch64", userAgentArch: "aarch64", extension: "flatpak" },
  { arch: "aarch64", userAgentArch: "arm64", extension: "flatpak" },
  { arch: "riscv64", userAgentArch: "riscv64", extension: "tar.gz" },
];

for (const { arch, userAgentArch, extension } of cases) {
  test.describe(`Linux ${userAgentArch} downloads`, () => {
    test.use({
      userAgent: `Mozilla/5.0 (X11; Linux ${userAgentArch}; rv:128.0) Gecko/20100101 Firefox/128.0`,
    });

    test("selects the matching launcher and standalone builds", async ({ page }) => {
      await page.goto("/apps/photocraft");
      const launcher = page.getByRole("link", { name: "Download Launcher" }).first();
      await expect(launcher).toHaveAttribute("href", new RegExp(`artcraft-launcher-.*-linux-${arch}\\.${extension.replace(".", "\\.")}$`));
      const card = page.locator("article").filter({ has: page.getByRole("heading", { name: "Linux", exact: true }) });
      await expect(card.getByText("Your system")).toBeVisible();
      await expect(card.getByRole("link", { name: "Standalone for Linux" }))
        .toHaveAttribute("href", new RegExp(`photocraft-.*-linux-${arch}\\.${extension.replace(".", "\\.")}$`));
      if (arch === "riscv64") {
        await page.getByRole("button", { name: "Other Linux download formats" }).first().click();
        const menu = page.getByRole("menu");
        await expect(menu.getByRole("menuitem")).toHaveCount(3);
        await expect(menu.getByRole("menuitem").filter({ hasText: "Tarball" })).toContainText("Default");
        await expect(menu.getByRole("menuitem").filter({ hasText: "Debian package" })).toHaveAttribute("href", /linux-riscv64\.deb$/);
        await expect(menu.getByRole("menuitem").filter({ hasText: "RPM package" })).toHaveAttribute("href", /linux-riscv64\.rpm$/);
      }
    });
  });
}

for (const architecture of ["arm", "riscv"]) {
  test.describe(`reduced Linux UA with ${architecture} hints`, () => {
    test.use({ userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130.0 Safari/537.36" });

    test("selects the hinted architecture for both downloads", async ({ page }) => {
      await page.addInitScript((architecture) => {
        Object.defineProperty(navigator, "userAgentData", {
          value: { getHighEntropyValues: async () => ({ architecture, bitness: "64" }) },
        });
      }, architecture);
      const arch = architecture === "arm" ? "aarch64" : "riscv64";
      const extension = architecture === "arm" ? "flatpak" : "tar\\.gz";
      await page.goto("/apps/photocraft");
      await expect(page.getByRole("link", { name: "Download Launcher", exact: true }).first())
        .toHaveAttribute("href", new RegExp(`linux-${arch}\\.${extension}$`));
      await expect(page.getByRole("link", { name: "Standalone for Linux", exact: true }))
        .toHaveAttribute("href", new RegExp(`linux-${arch}\\.${extension}$`));
    });
  });
}

for (const hintState of ["missing", "rejected", "empty"]) {
  test.describe(`Linux with ${hintState} client hints`, () => {
    test.use({ userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130.0 Safari/537.36" });

    test("keeps other launcher architectures reachable", async ({ page }) => {
      await page.addInitScript((hintState) => {
        const value = hintState === "missing" ? undefined : {
          getHighEntropyValues: async () => {
            if (hintState === "rejected") throw new Error("Client hints unavailable");
            return {};
          },
        };
        Object.defineProperty(navigator, "userAgentData", { value });
      }, hintState);
      await page.goto("/apps/photocraft");
      const allDownloads = page.getByRole("link", { name: "All launcher downloads (other architectures)", exact: true }).first();
      await expect(allDownloads).toBeVisible();
      await expect(allDownloads).toHaveAttribute("href", /^https:\/\/github\.com\/storytold\/craft-launcher\/releases\/tag\/v[^/]+$/);
      await expect(page.getByRole("link", { name: "Download Launcher", exact: true }).first())
        .toHaveAttribute("href", /linux-x86_64\.flatpak$/);
      // The full release page is a deliberate escape hatch, not the standalone app anchor.
      await allDownloads.focus();
      await expect(allDownloads).toBeFocused();
    });
  });
}

// Keep this in sync with the release updater's catalog extraction contract.
test("release updater includes every RISC-V package", () => {
  const source = readFileSync("src/lib/crafting-app-releases.ts", "utf8");
  const catalog = source.match(/export const CRAFT_RELEASE_ASSETS:[\s\S]*?=\s*\[([\s\S]*?)\n\];/);
  expect(catalog).not.toBeNull();
  const files = [...(catalog?.[1] ?? "").matchAll(/\bfile:\s*"([^"]+)"/g)].map(match => match[1]);
  expect(files.filter(file => file.includes("riscv64"))).toEqual([
    "{slug}-{version}-linux-riscv64.tar.gz",
    "{slug}-{version}-linux-riscv64.deb",
    "{slug}-{version}-linux-riscv64.rpm",
  ]);
});
