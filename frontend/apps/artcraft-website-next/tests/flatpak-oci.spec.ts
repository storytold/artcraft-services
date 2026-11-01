import { createHash } from "node:crypto";
import { expect, test } from "@playwright/test";
import { getFlatpakOciIndex } from "../src/lib/flatpak-oci";

const sha = (body: string) => `sha256:${createHash("sha256").update(body).digest("hex")}`;
const ref = "app/ai.storyteller.pdfcraft/x86_64/stable";
const config = JSON.stringify({ os: "linux", architecture: "amd64", config: { Labels: { "org.flatpak.ref": ref, "org.flatpak.metadata": "[Application]" } } });
const manifest = JSON.stringify({ schemaVersion: 2, mediaType: "application/vnd.oci.image.manifest.v1+json", config: { digest: sha(config), size: Buffer.byteLength(config) }, layers: [] });

function registry(options: { corruptConfig?: boolean; wrongDigest?: boolean; missing?: boolean } = {}): typeof fetch {
  return async (input) => {
    const url = String(input);
    if (url.startsWith("https://ghcr.io/token?")) return Response.json({ token: "test-token" });
    if (options.missing) return new Response(null, { status: 404 });
    if (url.includes("/manifests/")) {
      return new Response(manifest, { headers: { "docker-content-digest": options.wrongDigest ? `sha256:${"0".repeat(64)}` : sha(manifest) } });
    }
    if (url.includes("/blobs/")) return new Response(options.corruptConfig ? "{}" : config);
    throw new Error(`Unexpected URL: ${url}`);
  };
}

const pdf = new URLSearchParams({ repository: "storytold/pdfcraft", architecture: "amd64" });

test("indexes the actual manifest digest and verified Flatpak labels", async () => {
  const index = await getFlatpakOciIndex(pdf, registry());
  expect(index.Results).toHaveLength(1);
  const image = index.Results[0].Images[0];
  expect(image.Digest).toBe(sha(manifest));
  expect(image.Size).toBe(Buffer.byteLength(manifest));
  expect(image.Labels["org.flatpak.ref"]).toBe(ref);
  expect(image.Tags).toEqual(["0.16.1-x86_64"]);
});

test("does not advertise an image absent from the registry", async () => {
  const index = await getFlatpakOciIndex(pdf, registry({ missing: true }));
  expect(index.Results).toEqual([]);
});

test("rejects mismatched manifest and configuration digests", async () => {
  await expect(getFlatpakOciIndex(pdf, registry({ wrongDigest: true }))).rejects.toThrow("manifest digest mismatch");
  await expect(getFlatpakOciIndex(pdf, registry({ corruptConfig: true }))).rejects.toThrow("configuration digest mismatch");
});

test("filters on real tags and Flatpak references", async () => {
  const params = new URLSearchParams({ repository: "storytold/pdfcraft", architecture: "amd64", tag: "0.16.1-x86_64", "label:org.flatpak.ref": ref });
  expect((await getFlatpakOciIndex(params, registry())).Results).toHaveLength(1);
  params.set("tag", "stable");
  expect((await getFlatpakOciIndex(params, registry())).Results).toEqual([]);
});
