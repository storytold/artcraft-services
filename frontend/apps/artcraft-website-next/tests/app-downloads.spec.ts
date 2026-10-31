import { expect, test } from "@playwright/test";

const FREEBSD_UA = "Mozilla/5.0 (X11; FreeBSD amd64; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";
const GHOSTBSD_UA = "Mozilla/5.0 (X11; GhostBSD amd64) AppleWebKit/537.36 Chrome/130.0 Safari/537.36";
const LINUX_UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130.0 Safari/537.36";

for (const userAgent of [FREEBSD_UA, GHOSTBSD_UA]) {
  test.describe(`downloads for ${userAgent.includes("GhostBSD") ? "GhostBSD" : "FreeBSD"}`, () => {
    test.use({ userAgent });

    test("offers FreeBSD launcher and standalone downloads", async ({ page }) => {
      await page.goto("/apps/photocraft");
      const launcher = page.getByRole("link", { name: "Download Launcher" }).first();
      await expect(launcher).toHaveAttribute("href", /artcraft-launcher-.*-freebsd-x86_64\.tar\.gz$/);
      const card = page.locator("article").filter({ has: page.getByRole("heading", { name: "FreeBSD" }) });
      await expect(card.getByText("Your system")).toBeVisible();
      await expect(card.getByRole("link", { name: "Standalone for FreeBSD" }))
        .toHaveAttribute("href", /photocraft-.*-freebsd-x86_64\.tar\.gz$/);
    });
  });
}

test.describe("downloads for Linux", () => {
  test.use({ userAgent: LINUX_UA });

  test("continues to offer the Linux launcher", async ({ page }) => {
    await page.goto("/apps/photocraft");
    await expect(page.getByRole("link", { name: "Download Launcher" }).first())
      .toHaveAttribute("href", /artcraft-launcher-.*-linux-x86_64\.flatpak$/);
  });
});
