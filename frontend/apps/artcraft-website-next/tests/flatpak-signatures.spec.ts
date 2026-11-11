import { createHash } from "node:crypto";
import { expect, test } from "@playwright/test";
import * as openpgp from "openpgp";
import { GET as getFlatpakRepoDescriptor } from "../src/app/artcraft-oci.flatpakrepo/route";
import { GET as getFlatpakRef } from "../src/app/apps/[slug]/[filename]/route";
import { GET as getLookasideSignature } from "../src/app/oci-signatures/[...path]/route";
import {
  buildOciSignaturePayload,
  getFlatpakSignatureDescriptor,
  getOciImageSignature,
  parseSignatureLookasidePath,
} from "../src/lib/flatpak-signatures";
import { getFlatpakOciImageDigests } from "../src/lib/flatpak-oci";

const sha256 = (body: string) => `sha256:${createHash("sha256").update(body).digest("hex")}`;

/** Minimal registry: one manifest per repository/tag with the labels Flatpak expects. */
function fakeRegistry(): typeof fetch {
  const configs = new Map<string, string>();
  return async (input) => {
    const url = new URL(input instanceof Request ? input.url : input.toString());
    if (url.pathname === "/token") return Response.json({ token: "test-pull-token" });
    const manifest = url.pathname.match(/^\/v2\/(.+)\/manifests\/(.+)$/);
    if (manifest) {
      const repo = manifest[1];
      const tag = decodeURIComponent(manifest[2]);
      const slug = repo.split("/")[1];
      const appId = `ai.storyteller.${slug === "craft-launcher" ? "artcraft-launcher" : slug}`;
      const arch = tag.endsWith("-aarch64") ? "aarch64" : "x86_64";
      const config = JSON.stringify({
        config: {
          Labels: {
            "org.flatpak.ref": `app/${appId}/${arch}/stable`,
            "org.flatpak.metadata": `[Application]\nname=${appId}\n`,
          },
        },
      });
      configs.set(sha256(config), config);
      const body = JSON.stringify({
        schemaVersion: 2,
        mediaType: "application/vnd.oci.image.manifest.v1+json",
        config: { mediaType: "application/vnd.oci.image.config.v1+json", digest: sha256(config) },
      });
      return new Response(body, { headers: { "docker-content-digest": sha256(body) } });
    }
    const blob = url.pathname.match(/\/blobs\/(sha256:[a-f0-9]{64})$/);
    const config = blob && configs.get(blob[1]);
    if (config) return new Response(config);
    return new Response("not found", { status: 404 });
  };
}

const originalFetch = globalThis.fetch;
const originalRegistryUrl = process.env.FLATPAK_OCI_REGISTRY_URL;
const originalSigningKey = process.env.LINUX_SIGNING_KEY;

/** Read a containers/image lookaside file: a signed message around the atomic signature JSON. */
async function readSignedPayload(
  signed: Uint8Array,
  publicKeyArmored: string,
): Promise<{ critical: { type: string; image: { "docker-manifest-digest": string }; identity: { "docker-reference": string } } }> {
  const result = await openpgp.verify({
    message: await openpgp.readMessage({ binaryMessage: signed }),
    verificationKeys: await openpgp.readKey({ armoredKey: publicKeyArmored }),
  });
  await result.signatures[0].verified;
  return JSON.parse(Buffer.from(result.data as Uint8Array).toString("utf-8"));
}

test.describe("Flatpak OCI signatures", () => {
  let secretKey: string;
  let publicKeyArmored: string;

  test.beforeAll(async () => {
    const generated = await openpgp.generateKey({
      type: "curve25519",
      userIDs: [{ name: "ArtCraft Test", email: "test@example.com" }],
      format: "armored",
    });
    secretKey = generated.privateKey;
    publicKeyArmored = generated.publicKey;
  });

  test.beforeEach(() => {
    process.env.FLATPAK_OCI_REGISTRY_URL = "https://registry.test";
    process.env.LINUX_SIGNING_KEY = secretKey;
    globalThis.fetch = fakeRegistry();
  });

  test.afterEach(() => {
    globalThis.fetch = originalFetch;
    if (originalRegistryUrl === undefined) delete process.env.FLATPAK_OCI_REGISTRY_URL;
    else process.env.FLATPAK_OCI_REGISTRY_URL = originalRegistryUrl;
    if (originalSigningKey === undefined) delete process.env.LINUX_SIGNING_KEY;
    else process.env.LINUX_SIGNING_KEY = originalSigningKey;
  });

  test("names the manifest digest and the registry identity containers/image expects", () => {
    const digest = `sha256:${"a".repeat(64)}`;
    const payload = JSON.parse(
      buildOciSignaturePayload(digest, "storytold/pdfcraft", new Date("2026-11-05T00:00:00Z")),
    );

    expect(payload.critical.type).toBe("atomic container signature");
    expect(payload.critical.image["docker-manifest-digest"]).toBe(digest);
    expect(payload.critical.identity["docker-reference"]).toBe("registry.test/storytold/pdfcraft");
    expect(payload.optional.timestamp).toBe(1793836800);
  });

  test("parses the lookaside path Flatpak builds from the index repository name", () => {
    const digest = `sha256:${"b".repeat(64)}`;
    expect(
      parseSignatureLookasidePath(["storytold", "pdfcraft@sha256=" + "b".repeat(64), "signature-1"]),
    ).toEqual({ repository: "storytold/pdfcraft", digest, index: 1 });

    expect(parseSignatureLookasidePath(["storytold/pdfcraft@sha256=abc/signature-1"])).toBeNull();
    expect(parseSignatureLookasidePath(["storytold/pdfcraft", "signature"])).toBeNull();
    expect(parseSignatureLookasidePath(["../etc/passwd@sha256=" + "c".repeat(64), "signature-1"]))
      .toBeNull();
    expect(parseSignatureLookasidePath(["single@sha256=" + "c".repeat(64), "signature-1"])).toBeNull();
  });

  test("signs each published image digest", async () => {
    const digests = await getFlatpakOciImageDigests("storytold/pdfcraft");
    expect(digests).toHaveLength(2);

    const signature = await getOciImageSignature("storytold/pdfcraft", digests![0]);
    expect(signature).not.toBeNull();
    const payload = await readSignedPayload(signature!, publicKeyArmored);
    expect(payload.critical.image["docker-manifest-digest"]).toBe(digests![0]);
    expect(payload.critical.identity["docker-reference"]).toBe("registry.test/storytold/pdfcraft");
  });

  test("does not sign digests or repositories the index does not publish", async () => {
    expect(await getOciImageSignature("storytold/pdfcraft", `sha256:${"d".repeat(64)}`)).toBeNull();
    expect(await getOciImageSignature("other/app", `sha256:${"d".repeat(64)}`)).toBeNull();
  });

  test("serves verifiable signatures from the lookaside route", async () => {
    const [digest] = (await getFlatpakOciImageDigests("storytold/pdfcraft"))!;
    const response = await getLookasideSignature(
      new Request(`https://getartcraft.com/oci-signatures/storytold/pdfcraft@sha256=${digest.slice(7)}/signature-1`),
      { params: Promise.resolve({ path: ["storytold", `pdfcraft@sha256=${digest.slice(7)}`, "signature-1"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("application/pgp-signature");
    const payload = await readSignedPayload(
      new Uint8Array(await response.arrayBuffer()),
      publicKeyArmored,
    );
    expect(payload.critical.image["docker-manifest-digest"]).toBe(digest);

    // Only one signature exists per image; Flatpak stops asking when signature-2 is missing.
    const second = await getLookasideSignature(
      new Request(`https://getartcraft.com/oci-signatures/storytold/pdfcraft@sha256=${digest.slice(7)}/signature-2`),
      { params: Promise.resolve({ path: ["storytold", `pdfcraft@sha256=${digest.slice(7)}`, "signature-2"] }) },
    );
    expect(second.status).toBe(404);

    const unknown = await getLookasideSignature(
      new Request(`https://getartcraft.com/oci-signatures/other/app@sha256=${"e".repeat(64)}/signature-1`),
      { params: Promise.resolve({ path: ["other", `app@sha256=${"e".repeat(64)}`, "signature-1"] }) },
    );
    expect(unknown.status).toBe(404);
  });

  test("enables verification in the remote and single-app descriptors", async () => {
    const publicKey = await openpgp.readKey({ armoredKey: publicKeyArmored });
    const expectedKey = Buffer.from(publicKey.write()).toString("base64");

    const repo = await (
      await getFlatpakRepoDescriptor(new Request("https://getartcraft.com/artcraft-oci.flatpakrepo"))
    ).text();
    expect(repo).toContain("Url=oci+https://getartcraft.com");
    expect(repo).toContain(`GPGKey=${expectedKey}`);
    expect(repo).toContain("SignatureLookaside=https://getartcraft.com/oci-signatures");
    expect(repo).not.toContain("GPGVerify=false");

    const ref = await (
      await getFlatpakRef(
        new Request("https://getartcraft.com/apps/pdfcraft/pdfcraft-oci.flatpakref"),
        { params: Promise.resolve({ slug: "pdfcraft", filename: "pdfcraft-oci.flatpakref" }) },
      )
    ).text();
    expect(ref).toContain("Name=ai.storyteller.pdfcraft");
    expect(ref).toContain(`GPGKey=${expectedKey}`);
    expect(ref).toContain("SignatureLookaside=https://getartcraft.com/oci-signatures");
  });

  test("leaves the remote unsigned when no signing key is configured", async () => {
    delete process.env.LINUX_SIGNING_KEY;

    expect(await getFlatpakSignatureDescriptor("https://getartcraft.com")).toEqual([]);
    const repo = await (
      await getFlatpakRepoDescriptor(new Request("https://getartcraft.com/artcraft-oci.flatpakrepo"))
    ).text();
    expect(repo).toContain("GPGVerify=false");
    expect(repo).not.toContain("GPGKey=");

    // A repository whose digests were not signed earlier in this process, so the cache cannot
    // answer for a key that is no longer configured.
    const signature = await getOciImageSignature(
      "storytold/texturecraft",
      (await getFlatpakOciImageDigests("storytold/texturecraft"))![0],
    );
    expect(signature).toBeNull();
  });
});
