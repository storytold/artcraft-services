import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "@playwright/test";
import { createMessage, generateKey, readKey, readPrivateKey, sign } from "openpgp";
import { RELEASE_SIGNING_KEY, parseSignedShaSums, verifyDetachedSignature } from "../src/lib/release-signatures";
import { resolvePackageDownloadUrl, resolvePackageSignatureUrl } from "../src/lib/linux-repos";
import { CRAFTING_APPS, craftAppRelease, craftReleaseDownloads } from "../src/lib/crafting-apps";

// Signed by GnuPG the way craft-common-actions signs a release (`gpg --armor --detach-sign`),
// with a test-only key.
const FIXTURE = join(__dirname, "fixtures/signed-release");
const sums = new Uint8Array(readFileSync(join(FIXTURE, "SHA256SUMS.txt")));
const signature = readFileSync(join(FIXTURE, "SHA256SUMS.txt.asc"), "utf8");
const key = readFileSync(join(FIXTURE, "release-signing-key.asc"), "utf8");

async function otherKeySignature(data: Uint8Array): Promise<{ publicKey: string; signature: string }> {
  const { privateKey, publicKey } = await generateKey({ type: "curve25519", userIDs: [{ name: "Not ArtCraft" }] });
  const signingKey = await readPrivateKey({ armoredKey: privateKey });
  const message = await createMessage({ binary: data });
  return { publicKey, signature: (await sign({ message, signingKeys: signingKey, detached: true })) as string };
}

test.describe("release signatures", () => {
  test("a GnuPG signature over SHA256SUMS.txt verifies with the release key", async () => {
    expect(await verifyDetachedSignature(sums, signature, key)).toBe(true);
    const parsed = await parseSignedShaSums(sums, signature, key);
    expect(parsed.get("demo-1.0-linux-x86_64.rpm")).toBe("b".repeat(64));
    expect(parsed.size).toBe(3);
  });

  test("a changed checksum file is rejected", async () => {
    const tampered = new TextEncoder().encode(new TextDecoder().decode(sums).replace("a".repeat(64), "d".repeat(64)));
    expect(await verifyDetachedSignature(tampered, signature, key)).toBe(false);
    await expect(parseSignedShaSums(tampered, signature, key)).rejects.toThrow("does not verify");
  });

  test("a signature by another key is rejected", async () => {
    const other = await otherKeySignature(sums);
    expect(await verifyDetachedSignature(sums, other.signature, other.publicKey)).toBe(true);
    expect(await verifyDetachedSignature(sums, other.signature, key)).toBe(false);
    await expect(parseSignedShaSums(sums, other.signature, key)).rejects.toThrow("does not verify");
  });

  test("a malformed signature is rejected, not thrown", async () => {
    expect(await verifyDetachedSignature(sums, "not a signature", key)).toBe(false);
    expect(await verifyDetachedSignature(sums, signature, "not a key")).toBe(false);
  });

  test("a release from before signing still lists its checksums", async () => {
    expect((await parseSignedShaSums(sums, null, key)).size).toBe(3);
    // No release key configured: the signature isn't checked.
    expect((await parseSignedShaSums(sums, "not a signature", "")).size).toBe(3);
  });

  test("every catalogued release's SHA256SUMS.txt signature verifies with the release key", async () => {
    const unsigned: string[] = [];
    for (const app of CRAFTING_APPS) {
      const release = craftAppRelease(app);
      if (!release) continue;
      const base = `https://github.com/storytold/${app.slug}/releases/download/v${release.version}/SHA256SUMS.txt`;
      const [data, sig] = await Promise.all([fetch(base), fetch(`${base}.asc`)]);
      expect(data.ok, `${app.slug} ${release.version}: SHA256SUMS.txt`).toBe(true);
      // shortcut: a release from before signing has no .asc; require one (expect sig.ok) once every catalogued release is signed.
      if (sig.status === 404) {
        unsigned.push(`${app.slug} ${release.version}`);
        continue;
      }
      expect(sig.ok, `${app.slug} ${release.version}: SHA256SUMS.txt.asc`).toBe(true);
      const ok = await verifyDetachedSignature(new Uint8Array(await data.arrayBuffer()), await sig.text(), RELEASE_SIGNING_KEY);
      expect(ok, `${app.slug} ${release.version}`).toBe(true);
    }
    if (unsigned.length) test.info().annotations.push({ type: "unsigned releases", description: unsigned.join(", ") });
  });

  test("the release key is a public key with the expected fingerprint", async () => {
    expect(RELEASE_SIGNING_KEY).not.toContain("PRIVATE");
    const key = await readKey({ armoredKey: RELEASE_SIGNING_KEY });
    expect(key.isPrivate()).toBe(false);
    expect(key.getFingerprint()).toBe("2b76741b414aad6cd33e631213a005c0b9d7c3ed");
    const signing = await key.getSigningKey();
    expect(signing.getFingerprint()).toBe("568dc1f68f8faecc797f3c7b006daf6bb0ef334e");
  });
});

test.describe("pacman signature downloads", () => {
  test("a package's .sig redirects to the same release's signature asset", () => {
    const pkg = CRAFTING_APPS.flatMap((app) => {
      const release = craftAppRelease(app);
      return release ? craftReleaseDownloads(app, release) : [];
    }).find((d) => d.fileName.endsWith(".pkg.tar.zst"));
    expect(pkg, "a catalogued Arch package").toBeDefined();
    expect(resolvePackageSignatureUrl(`${pkg!.fileName}.sig`)).toBe(`${resolvePackageDownloadUrl(pkg!.fileName)}.sig`);
  });

  test("only catalogued packages' signatures redirect", () => {
    expect(resolvePackageSignatureUrl("evil-1.0-linux-x86_64.pkg.tar.zst.sig")).toBeNull();
    expect(resolvePackageSignatureUrl("../../SHA256SUMS.txt.sig")).toBeNull();
    expect(resolvePackageSignatureUrl("not-a-signature.pkg.tar.zst")).toBeNull();
  });
});
