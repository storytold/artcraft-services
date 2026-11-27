import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash, generateKeyPairSync, verify } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  MAX_CATALOG_BYTES,
  enrichLauncherCatalog,
  generateLauncherCatalog,
  parseSha256Sums,
} from "./generate-launcher-catalog.mjs";
import { signLauncherCatalog } from "./sign-launcher-catalog.mjs";

const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);
const HASH_C = "c".repeat(64);

const BASE_TOML = `# Header comment
# Source: test

[[apps]]
id = "photocraft"
name = "PhotoCraft"
repo = "storytold/photocraft"
[apps.details]
headline = "Image editor"
[[apps.details.features]]
title = "Fast"
text = "Native GPU"

[[apps]]
id = "solidcraft"
name = "SolidCraft"
repo = "tangency-labs/solidcraft"
[apps.details]
headline = "CAD"
`;

test("parseSha256Sums parses standard and binary-mode entries and rejects unsafe filenames", () => {
  const sums = parseSha256Sums([
    `${HASH_B.toUpperCase()}  photocraft-0.23.1-windows-x64.msi`,
    `${HASH_A} *photocraft-0.23.1-macos-universal.dmg`,
    `${HASH_C}  ../evil.msi`,
    `${HASH_C}  .hidden`,
    "not-a-hash  file.zip",
    `${HASH_C}  sub/dir.zip`,
  ].join("\n"));

  assert.deepEqual(sums, [
    { name: "photocraft-0.23.1-macos-universal.dmg", sha256: HASH_A },
    { name: "photocraft-0.23.1-windows-x64.msi", sha256: HASH_B },
  ]);
});

test("generateLauncherCatalog enriches stable, prerelease, and nightly channels and produces valid signed TOML", async (t) => {
  const dir = mkdtempSync(join(tmpdir(), "launcher-catalog-gen-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));

  const baseCatalogPath = join(dir, "base.toml");
  const outCatalogPath = join(dir, "launcher-catalog.toml");
  writeFileSync(baseCatalogPath, BASE_TOML, "utf8");
  writeFileSync(join(dir, "launcher-announcements.toml"), '[[announcements]]\nid = "a1"\ntitle = "Hi"\n', "utf8");

  const responses = new Map([
    [
      "https://github.com/storytold/craft-launcher/releases/download/v0.20.3/SHA256SUMS.txt",
      `${HASH_A}  artcraft-launcher-0.20.3-macos-universal.dmg\n`,
    ],
    [
      "https://github.com/storytold/craft-launcher/releases/download/v0.21.0-nightly.20261126.1.1/SHA256SUMS.txt",
      `${HASH_B}  artcraft-launcher-0.21.0-nightly.20261126.1.1-linux-x86_64.AppImage\n`,
    ],
    [
      "https://github.com/storytold/photocraft/releases/download/v0.23.1/SHA256SUMS.txt",
      `${HASH_A}  photocraft-0.23.1-macos-universal.dmg\n${HASH_B}  photocraft-0.23.1-windows-x64.msi\n`,
    ],
    [
      "https://github.com/storytold/photocraft/releases/download/v0.24.0-rc.1/SHA256SUMS.txt",
      `${HASH_C}  photocraft-0.24.0-rc.1-macos-universal.dmg\n`,
    ],
    [
      "https://github.com/storytold/photocraft/releases/download/v0.24.0-nightly.20261126.5.1/SHA256SUMS.txt",
      `${HASH_C}  photocraft-0.24.0-nightly.20261126.5.1-linux-x86_64.AppImage\n`,
    ],
  ]);

  const fakeFetch = async (url) => {
    const body = responses.get(String(url));
    if (body === undefined) {
      return { ok: false, status: 404, text: async () => "" };
    }
    return { ok: true, status: 200, arrayBuffer: async () => Buffer.from(body, "utf8") };
  };

  const result = await generateLauncherCatalog({
    baseCatalogPath,
    outCatalogPath,
    generatedAt: "2026-11-26T21:30:00.000Z",
    targets: [
      {
        id: "artcraft-launcher",
        repo: "storytold/craft-launcher",
        channel: "stable",
        tag: "v0.20.3",
        version: "0.20.3",
        prerelease: false,
        required: true,
      },
      {
        id: "artcraft-launcher",
        repo: "storytold/craft-launcher",
        channel: "nightly",
        tag: "v0.21.0-nightly.20261126.1.1",
        version: "0.21.0-nightly.20261126.1.1",
        prerelease: true,
        required: false,
      },
      {
        id: "photocraft",
        repo: "storytold/photocraft",
        channel: "stable",
        tag: "v0.23.1",
        version: "0.23.1",
        prerelease: false,
        required: true,
      },
      {
        id: "photocraft",
        repo: "storytold/photocraft",
        channel: "prerelease",
        tag: "v0.24.0-rc.1",
        version: "0.24.0-rc.1",
        prerelease: true,
        required: false,
      },
      {
        id: "photocraft",
        repo: "storytold/photocraft",
        channel: "nightly",
        tag: "v0.24.0-nightly.20261126.5.1",
        version: "0.24.0-nightly.20261126.5.1",
        prerelease: true,
        required: false,
      },
      {
        id: "solidcraft",
        repo: "tangency-labs/solidcraft",
        channel: "stable",
        tag: "v0.3.0",
        version: "0.3.0",
        prerelease: false,
        required: false,
      },
    ],
    fetchImpl: fakeFetch,
  });

  assert.equal(result.releaseCount, 6);
  assert.ok(result.bytes > 0 && result.bytes <= MAX_CATALOG_BYTES);

  // Parse the generated TOML with Python's stdlib tomllib to verify strict TOML 1.0 compliance.
  const py = spawnSync(
    "python3",
    [
      "-c",
      'import json, sys, tomllib; print(json.dumps(tomllib.loads(open(sys.argv[1], "rb").read().decode())))',
      outCatalogPath,
    ],
    { encoding: "utf8" },
  );
  assert.equal(py.status, 0, py.stderr);
  const parsed = JSON.parse(py.stdout);

  assert.equal(parsed.generated_at, "2026-11-26T21:30:00.000Z");
  assert.equal(parsed.launcher.id, "artcraft-launcher");
  assert.equal(parsed.launcher.releases.stable.tag, "v0.20.3");
  assert.equal(parsed.launcher.releases.stable.assets.length, 2);
  assert.equal(parsed.launcher.releases.nightly.tag, "v0.21.0-nightly.20261126.1.1");
  assert.equal(parsed.launcher.releases.nightly.prerelease, true);

  assert.equal(parsed.apps.length, 2);
  const photo = parsed.apps[0];
  assert.equal(photo.id, "photocraft");
  assert.equal(photo.releases.stable.tag, "v0.23.1");
  assert.equal(photo.releases.stable.assets.length, 3);
  assert.equal(photo.releases.stable.assets[0].sha256, HASH_A);
  // The checksum document is listed too, hashed over the exact bytes that were fetched.
  const sums = photo.releases.stable.assets.find((a) => a.name === "SHA256SUMS.txt");
  assert.equal(
    sums.download_url,
    "https://github.com/storytold/photocraft/releases/download/v0.23.1/SHA256SUMS.txt",
  );
  assert.equal(
    sums.sha256,
    createHash("sha256").update(`${HASH_A}  photocraft-0.23.1-macos-universal.dmg\n${HASH_B}  photocraft-0.23.1-windows-x64.msi\n`).digest("hex"),
  );
  assert.equal(photo.releases.prerelease.tag, "v0.24.0-rc.1");
  assert.equal(photo.releases.nightly.tag, "v0.24.0-nightly.20261126.5.1");

  // Community app with missing SHA256SUMS.txt still emits channel metadata without failing the build.
  const solid = parsed.apps[1];
  assert.equal(solid.id, "solidcraft");
  assert.equal(solid.releases.stable.tag, "v0.3.0");
  assert.equal(solid.releases.stable.assets, undefined);

  // Signing the generated catalog produces a valid Ed25519 detached signature over the final bytes.
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const env = {
    LAUNCHER_CATALOG_SIGNING_KEY: Buffer.from(privateKey.export({ type: "pkcs8", format: "pem" })).toString("base64"),
  };
  signLauncherCatalog(dir, env);
  const finalBytes = readFileSync(outCatalogPath);
  const sig = Buffer.from(readFileSync(join(dir, "launcher-catalog.toml.sig"), "utf8").trim(), "base64");
  assert.equal(sig.length, 64);
  assert.equal(verify(null, finalBytes, publicKey, sig), true);
});

test("enrichLauncherCatalog rejects outputs larger than MAX_CATALOG_BYTES", () => {
  const hugeAssets = Array.from({ length: 6000 }, (_, i) => ({
    name: `photocraft-0.23.1-asset-${i}.tar.gz`,
    download_url: `https://github.com/storytold/photocraft/releases/download/v0.23.1/photocraft-0.23.1-asset-${i}.tar.gz`,
    sha256: HASH_A,
  }));
  assert.throws(
    () =>
      enrichLauncherCatalog(BASE_TOML, {
        generatedAt: "2026-11-26T21:30:00.000Z",
        launcherReleases: {},
        appReleasesById: {
          photocraft: {
            stable: {
              tag: "v0.23.1",
              version: "0.23.1",
              html_url: "https://github.com/storytold/photocraft/releases/tag/v0.23.1",
              prerelease: false,
              assets: hugeAssets,
            },
          },
        },
      }),
    /MAX_CATALOG_BYTES/,
  );
});
