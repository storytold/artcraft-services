import { createHash } from "node:crypto";
import { expect, test } from "@playwright/test";
import { GET as getFlatpakRef } from "../src/app/apps/[slug]/[filename]/route";
import { GET as getDynamicIndex } from "../src/app/index/dynamic/route";
import { GET as getStaticIndex } from "../src/app/index/static/route";
import { getFlatpakOciAppIds, getFlatpakOciIndex } from "../src/lib/flatpak-oci";

const manifestDigest = (tag: string) =>
  `sha256:${createHash("sha256").update(`manifest:${tag}`).digest("hex")}`;
const originalFetch = globalThis.fetch;
const originalRegistryUrl = process.env.FLATPAK_OCI_REGISTRY_URL;
const originalReadToken = process.env.FLATPAK_OCI_READ_TOKEN;

test.describe("Flatpak OCI Registry Index Protocol", () => {
  test("serves app flatpakref files at their public URLs", async () => {
    const response = await getFlatpakRef(
      new Request("https://getartcraft.com/apps/pdfcraft/pdfcraft-oci.flatpakref"),
      { params: Promise.resolve({ slug: "pdfcraft", filename: "pdfcraft-oci.flatpakref" }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("application/x-flatpak-ref");
    expect(response.headers.get("content-disposition")).toContain('filename="pdfcraft-oci.flatpakref"');
    expect(await response.text()).toContain("Name=ai.storyteller.pdfcraft");
  });

  test("rejects a flatpakref filename that does not match its app slug", async () => {
    const response = await getFlatpakRef(
      new Request("https://getartcraft.com/apps/pdfcraft/other-oci.flatpakref"),
      { params: Promise.resolve({ slug: "pdfcraft", filename: "other-oci.flatpakref" }) },
    );

    expect(response.status).toBe(404);
  });

  test.beforeEach(() => {
    process.env.FLATPAK_OCI_REGISTRY_URL = "https://registry.test";
    delete process.env.FLATPAK_OCI_READ_TOKEN;
    globalThis.fetch = async (input, init) => {
      const url = new URL(input instanceof Request ? input.url : input.toString());
      if (url.pathname === "/token") {
        return Response.json({ token: "test-pull-token" });
      }
      if (init?.method === "HEAD") {
        const tag = decodeURIComponent(url.pathname.split("/").at(-1) ?? "");
        return new Response(null, {
          status: 200,
          headers: { "docker-content-digest": manifestDigest(tag) },
        });
      }
      return new Response("not found", { status: 404 });
    };
  });

  test.afterEach(() => {
    globalThis.fetch = originalFetch;
    if (originalRegistryUrl === undefined) delete process.env.FLATPAK_OCI_REGISTRY_URL;
    else process.env.FLATPAK_OCI_REGISTRY_URL = originalRegistryUrl;
    if (originalReadToken === undefined) delete process.env.FLATPAK_OCI_READ_TOKEN;
    else process.env.FLATPAK_OCI_READ_TOKEN = originalReadToken;
  });

  test("generates valid registry index response using real OCI manifest digests", async () => {
    const index = await getFlatpakOciIndex();
    expect(index.Registry).toBe("https://registry.test");
    expect(index.Results.length).toBeGreaterThan(0);

    const launcher = index.Results.find((r) => r.Repository === "storytold/craft-launcher");
    expect(launcher).toBeDefined();
    expect(launcher?.Images).toHaveLength(2);
    expect(launcher?.Images[0].Labels["org.flatpak.ref"]).toBe(
      "app/ai.storyteller.artcraft-launcher/x86_64/stable",
    );
    expect(launcher?.Images[0].Tags).toContain("stable-x86_64");

    const pdfcraft = index.Results.find((r) => r.Name === "storytold/pdfcraft");
    expect(pdfcraft).toBeDefined();
    expect(pdfcraft?.Repository).toBe("storytold/pdfcraft");
    const pdfcraftX86 = pdfcraft?.Images.find((img) => img.Architecture === "amd64");
    expect(pdfcraftX86?.Labels["org.flatpak.ref"]).toBe("app/ai.storyteller.pdfcraft/x86_64/stable");
    expect(pdfcraftX86?.Digest).toMatch(/^sha256:[a-f0-9]{64}$/);
    expect(pdfcraftX86?.Labels["org.flatpak.metadata"]).toContain("[Application]");
    expect(pdfcraftX86?.Labels["org.flatpak.metadata"]).toContain("ai.storyteller.pdfcraft");

    // `flatpak remote-ls` takes Version and Name from this AppStream label.
    const appdata = pdfcraftX86?.Labels["org.freedesktop.appstream.appdata"] ?? "";
    expect(appdata).toContain("<id>ai.storyteller.pdfcraft</id>");
    expect(appdata).toContain('<bundle type="flatpak">app/ai.storyteller.pdfcraft/x86_64/stable</bundle>');
    expect(appdata).toContain(`<release version="${pdfcraftX86?.Labels["org.opencontainers.image.version"]}"/>`);

    // The website's catalog owns release versions; no OCI build exists for Craft Anything yet.
    expect(index.Results.find((r) => r.Repository === "storytold/craft-anything")).toBeUndefined();
  });

  test("filters by architecture and repository before resolving manifests", async () => {
    const params = new URLSearchParams({ architecture: "amd64", repository: "storytold/pdfcraft" });
    const index = await getFlatpakOciIndex(params);

    expect(index.Results).toHaveLength(1);
    expect(index.Results[0].Images).toHaveLength(1);
    expect(index.Results[0].Images[0].Architecture).toBe("amd64");
  });

  test("filters by org.flatpak.ref label", async () => {
    const params = new URLSearchParams({
      "label:org.flatpak.ref": "app/ai.storyteller.pdfcraft/x86_64/stable",
    });
    const index = await getFlatpakOciIndex(params);

    expect(index.Results).toHaveLength(1);
    expect(index.Results[0].Repository).toBe("storytold/pdfcraft");
    expect(index.Results[0].Images).toHaveLength(1);
    expect(index.Results[0].Images[0].Architecture).toBe("amd64");
  });

  test("matches Flatpak's stable query to the architecture-specific GHCR tag", async () => {
    const params = new URLSearchParams({
      architecture: "amd64",
      tag: "stable",
      "label:org.flatpak.ref:exists": "1",
    });
    const index = await getFlatpakOciIndex(params);

    expect(index.Results.length).toBeGreaterThan(0);
    expect(index.Results.every((repo) => repo.Images.every((image) =>
      image.Labels["org.flatpak.ref"].endsWith("/stable"),
    ))).toBe(true);
  });

  test("answers Flatpak's default `latest` tag query for a remote URL without a fragment", async () => {
    const params = new URLSearchParams({
      architecture: "amd64",
      os: "linux",
      tag: "latest",
      "label:org.flatpak.ref:exists": "1",
    });
    const index = await getFlatpakOciIndex(params);

    expect(index.Results.length).toBeGreaterThan(0);
    expect(index.Results.every((repo) => repo.Images.every((image) => image.Architecture === "amd64"))).toBe(true);
  });

  test("reports missing registry tags rather than returning synthetic digests", async () => {
    process.env.FLATPAK_OCI_REGISTRY_URL = "https://missing.registry.test";
    globalThis.fetch = async (input, init) => {
      const url = new URL(input instanceof Request ? input.url : input.toString());
      if (url.pathname === "/token") return Response.json({ token: "test-pull-token" });
      if (init?.method === "HEAD") return new Response(null, { status: 404 });
      return new Response("not found", { status: 404 });
    };

    await expect(getFlatpakOciIndex(new URLSearchParams({ repository: "storytold/pdfcraft" })))
      .rejects.toThrow(/release workflow pushed it/);
  });

  test("lists expected app IDs from the catalog without querying GHCR", () => {
    globalThis.fetch = async () => {
      throw new Error("the app list should not query GHCR");
    };

    expect(getFlatpakOciAppIds()).toContain("ai.storyteller.artcraft-launcher");
    expect(getFlatpakOciAppIds()).toContain("ai.storyteller.pdfcraft");
  });

  test("index routes preserve query filters and declare Netlify query variation", async () => {
    const staticResponse = await getStaticIndex(
      new Request("https://getartcraft.com/index/static?architecture=arm64"),
    );
    expect(staticResponse.status).toBe(200);
    expect(staticResponse.headers.get("netlify-vary")).toBe("query");
    const staticIndex = await staticResponse.json();
    const staticImages = staticIndex.Results.flatMap((repo: { Images: { Architecture: string }[] }) => repo.Images);
    expect(staticImages.length).toBeGreaterThan(0);
    expect(staticImages.every((image: { Architecture: string }) => image.Architecture === "arm64")).toBe(true);

    const dynamicResponse = await getDynamicIndex(
      new Request("https://getartcraft.com/index/dynamic?architecture=amd64"),
    );
    expect(dynamicResponse.status).toBe(200);
    expect(dynamicResponse.headers.get("netlify-vary")).toBe("query");
    const dynamicIndex = await dynamicResponse.json();
    const dynamicImages = dynamicIndex.Results.flatMap((repo: { Images: { Architecture: string }[] }) => repo.Images);
    expect(dynamicImages.length).toBeGreaterThan(0);
    expect(dynamicImages.every((image: { Architecture: string }) => image.Architecture === "amd64")).toBe(true);
  });
});
