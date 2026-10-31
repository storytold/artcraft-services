import { expect, test } from "@playwright/test";
import { getFlatpakOciIndex } from "../src/lib/flatpak-oci";

test.describe("Flatpak OCI Registry Index Protocol", () => {
  test("generates valid registry index response", () => {
    const index = getFlatpakOciIndex();
    expect(index.Registry).toBe("https://ghcr.io/v2");
    expect(index.Results.length).toBeGreaterThan(0);

    // Verifies ArtCraft Launcher is present
    const launcher = index.Results.find((r) => r.Repository === "storytold/artcraft-launcher");
    expect(launcher).toBeDefined();
    expect(launcher?.Images).toHaveLength(2); // x86_64 and aarch64

    // Verifies PdfCraft is present
    const pdfcraft = index.Results.find((r) => r.Name === "storytold/pdfcraft");
    expect(pdfcraft).toBeDefined();
    expect(pdfcraft?.Repository).toBe("storytold/pdfcraft");

    const pdfcraftX86 = pdfcraft?.Images.find((img) => img.Architecture === "x86_64");
    expect(pdfcraftX86).toBeDefined();
    expect(pdfcraftX86?.Labels["org.flatpak.ref"]).toBe("app/ai.storyteller.pdfcraft/x86_64/stable");
  });

  test("filters by architecture", () => {
    const params = new URLSearchParams({ architecture: "amd64" });
    const index = getFlatpakOciIndex(params);

    for (const repo of index.Results) {
      for (const img of repo.Images) {
        expect(img.Architecture).toBe("x86_64");
      }
    }
  });

  test("filters by org.flatpak.ref label", () => {
    const params = new URLSearchParams({
      "label:org.flatpak.ref": "app/ai.storyteller.pdfcraft/x86_64/stable",
    });
    const index = getFlatpakOciIndex(params);

    expect(index.Results).toHaveLength(1);
    expect(index.Results[0].Repository).toBe("storytold/pdfcraft");
    expect(index.Results[0].Images).toHaveLength(1);
    expect(index.Results[0].Images[0].Architecture).toBe("x86_64");
  });

  test("filters by repository", () => {
    const params = new URLSearchParams({ repository: "storytold/pdfcraft" });
    const index = getFlatpakOciIndex(params);

    expect(index.Results).toHaveLength(1);
    expect(index.Results[0].Name).toBe("storytold/pdfcraft");
    expect(index.Results[0].Repository).toBe("storytold/pdfcraft");
  });
});
