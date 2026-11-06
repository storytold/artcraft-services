import { expect, test } from "@playwright/test";

for (const arch of ["x86_64", "aarch64", "riscv64"]) {
  test.describe(`Studio ${arch} primary download`, () => {
    test.use({ userAgent: `Mozilla/5.0 (X11; Linux ${arch}; rv:128.0) Gecko/20100101 Firefox/128.0` });
    test("offers the matching Linux build instead of Windows", async ({ page }) => {
      await page.goto("/download");
      const format = arch === "riscv64" ? "tar.gz" : "flatpak";
      await expect(page.getByRole("link", { name: "Download for Linux", exact: true }))
        .toHaveAttribute("href", `https://github.com/storytold/artcraft/releases/download/artcraft-v0.42.0/artcraft-0.42.0-linux-${arch}.${format}`);
    });
  });
}
