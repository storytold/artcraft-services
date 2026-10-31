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

test.describe("reduced Linux user agent", () => {
  test.use({ userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130.0 Safari/537.36" });

  test("uses 64-bit ARM client hints", async ({ page }) => {
    await page.addInitScript(() => {
      Object.defineProperty(navigator, "userAgentData", {
        value: { getHighEntropyValues: async () => ({ architecture: "arm", bitness: "64" }) },
      });
    });
    await page.goto("/apps/photocraft");
    await expect(page.getByRole("link", { name: "Download Launcher" }).first())
      .toHaveAttribute("href", /artcraft-launcher-.*-linux-aarch64\.flatpak$/);
  });
});
