import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash, generateKeyPairSync, verify } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  MAX_CATALOG_BYTES,
  collectConfiguredReleases,
  enrichLauncherCatalog,
  generateLauncherCatalog,
  matchPublishedAssets,
  parseSha256Sums,
} from "./generate-launcher-catalog.mjs";
import { signLauncherCatalog } from "./sign-launcher-catalog.mjs";
import { selectDeveloperNightly } from "./select-developer-nightly.mjs";

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

// The canonical catalog names UCSLabs, so publishing URLs under its former owner
// leaves the launcher with no usable assets even though the release exists.
test("MailCraft release targets and updater use its canonical repository", async () => {
  const targets = (await collectConfiguredReleases()).filter((target) => target.id === "mailcraft");
  assert.ok(targets.length > 0);
  for (const target of targets) assert.equal(target.repo, "UCSLabs/mailcraft");
  const workflow = readFileSync(new URL("../../../../.github/workflows/update-craft-release-versions.yml", import.meta.url), "utf8");
  assert.match(workflow, /id: "mailcraft", repo: "UCSLabs\/mailcraft"/);
});

// Model the public portable CADCraft release, including descriptor and checksum-list metadata.
const CAD_VERSION = "0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c";
const CAD_TAG = `v${CAD_VERSION}`;
const CAD_REPO = "storytold/cadcraft";
const CAD_PUBLISHED = "2026-11-27T22:48:29Z";

function developerRelease(version, suffixes, published = CAD_PUBLISHED) {
  const tag = `v${version}`;
  const names = [...suffixes.map(suffix => `cadcraft-${version}-${suffix}`), "NIGHTLY.json", "SHA256SUMS.txt"];
  const checksummed = names.filter(name => name !== "SHA256SUMS.txt");
  const sums = checksummed.map(name => `${HASH_A}  ${name}\n`).join("");
  return {
    sums,
    release: {
      tag_name: tag, published_at: published, draft: false, prerelease: true,
      assets: names.map((name, i) => ({
        name, state: "uploaded",
        size: name === "SHA256SUMS.txt" ? Buffer.byteLength(sums) : 1000 + i,
        digest: `sha256:${name === "SHA256SUMS.txt" ? createHash("sha256").update(sums).digest("hex") : HASH_A}`,
        browser_download_url: `https://github.com/${CAD_REPO}/releases/download/${tag}/${name}`,
      })),
    },
  };
}

test("canonical developer discovery accepts portable builds independently on every published platform", () => {
  for (const suffix of ["windows-x86_64-nightly.zip", "macos-aarch64-nightly.zip", "linux-x86_64-nightly.tar.gz"]) {
    const { release } = developerRelease(CAD_VERSION, [suffix]);
    const selected = selectDeveloperNightly([release], CAD_REPO, "cadcraft");
    assert.equal(selected.tag, CAD_TAG);
    assert.equal(selected.publishedAt, CAD_PUBLISHED);
    assert.equal(selected.assets.find(asset => asset.name.endsWith(suffix)).size, 1000);
  }
  const older = developerRelease("0.20.1-nightly.20261127.9.1.gf7adf3ad561c", ["macos-aarch64-nightly.zip"], "2026-11-28T00:00:00Z").release;
  const current = developerRelease(CAD_VERSION, ["windows-x86_64-nightly.zip"]).release;
  const draft = developerRelease("0.21.0-nightly.20261127.1.1.gf7adf3ad561c", ["macos-aarch64-nightly.zip"]).release;
  draft.draft = true;
  const legacy = developerRelease("0.30.0-nightly.20261127.1.1.gf7adf3ad561c", ["linux-x86_64.AppImage"]).release;
  // Legacy package-only releases remain handled by the separate distro selector.
  legacy.tag_name = "nightly-20261127";
  assert.equal(selectDeveloperNightly([draft, older, current, legacy], CAD_REPO, "cadcraft").tag, CAD_TAG);
  assert.equal(selectDeveloperNightly([legacy], CAD_REPO, "cadcraft"), null);
  const incomplete = developerRelease("0.21.0-nightly.20261127.1.1.gf7adf3ad561c", ["macos-aarch64-nightly.zip"]).release;
  incomplete.assets = incomplete.assets.filter(asset => asset.name !== "NIGHTLY.json");
  assert.equal(selectDeveloperNightly([current, incomplete], CAD_REPO, "cadcraft"), null);
});

test("signed catalog publishes exact release publication time and checksummed portable asset sizes", async t => {
  const dir = mkdtempSync(join(tmpdir(), "launcher-nightly-gen-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const baseCatalogPath = join(dir, "base.toml");
  const outCatalogPath = join(dir, "launcher-catalog.toml");
  writeFileSync(baseCatalogPath, '[[apps]]\nid = "cadcraft"\nname = "CADCraft"\nrepo = "storytold/cadcraft"\n');
  const { release, sums } = developerRelease(CAD_VERSION, [
    "linux-x86_64-nightly.tar.gz", "linux-x86_64-nightly.tar.gz.asc",
    "macos-aarch64-nightly.zip", "windows-x86_64-nightly.zip",
  ]);
  const selected = selectDeveloperNightly([release], CAD_REPO, "cadcraft");
  const target = {
    id: "cadcraft", repo: CAD_REPO, channel: "nightly", tag: selected.tag, version: selected.version,
    prerelease: true, required: false, publishedAt: selected.publishedAt, publishedAssets: selected.assets,
  };
  const fetchImpl = async url => {
    assert.equal(url, `https://github.com/${CAD_REPO}/releases/download/${CAD_TAG}/SHA256SUMS.txt`);
    return { ok: true, status: 200, arrayBuffer: async () => Buffer.from(sums) };
  };
  await generateLauncherCatalog({ baseCatalogPath, outCatalogPath, targets: [target], fetchImpl });
  const readNightly = () => {
    const py = spawnSync("python3", ["-c",
      'import json, sys, tomllib; print(json.dumps(tomllib.load(open(sys.argv[1], "rb"))["apps"][0]["releases"]["nightly"]))',
      outCatalogPath], { encoding: "utf8" });
    assert.equal(py.status, 0, py.stderr);
    return JSON.parse(py.stdout);
  };
  const nightly = readNightly();
  assert.equal(nightly.tag, CAD_TAG);
  assert.equal(nightly.published_at, CAD_PUBLISHED);
  assert.equal(nightly.assets.length, release.assets.length);
  for (const actual of release.assets) {
    const published = nightly.assets.find(asset => asset.name === actual.name);
    assert.equal(published.download_url, actual.browser_download_url);
    assert.equal(published.size_bytes, actual.size);
    assert.equal(published.sha256, actual.digest.slice(7));
  }
  // If the release metadata doesn't agree with the checksum list, don't emit a trusted-looking
  // nightly entry. The client must fall back to its existing discovery and verification path.
  const mismatch = target.publishedAssets.map(asset => asset.name === "NIGHTLY.json" ? { ...asset, sha256: HASH_B } : asset);
  await generateLauncherCatalog({
    baseCatalogPath, outCatalogPath, targets: [{ ...target, publishedAssets: mismatch }], fetchImpl,
  });
  assert.equal(readNightly().published_at, undefined);
  assert.equal(readNightly().assets, undefined);
});

test("matching release metadata rejects absent assets, missing sizes, and inconsistent checksum size", () => {
  const asset = { name: "SHA256SUMS.txt", sha256: HASH_A, size_bytes: 17 };
  for (const published of [[], [{ name: asset.name, size: 0 }], [{ name: asset.name, size: 18 }]]) {
    assert.throws(() => matchPublishedAssets([asset], published), /disagrees/);
  }
});

test("one checksummed portable platform is sufficient, but a descriptor alone is not", async t => {
  const dir = mkdtempSync(join(tmpdir(), "launcher-nightly-coverage-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const baseCatalogPath = join(dir, "base.toml");
  const outCatalogPath = join(dir, "launcher-catalog.toml");
  writeFileSync(baseCatalogPath, '[[apps]]\nid = "cadcraft"\nname = "CADCraft"\nrepo = "storytold/cadcraft"\n');
  const { release, sums } = developerRelease(CAD_VERSION, ["windows-x86_64-nightly.zip"]);
  async function generate(checksums) {
    // Keep the metadata for SHA256SUMS consistent with the manifest actually fetched:
    // missing package evidence, not a size/digest disagreement, must drive fallback.
    const actual = {
      ...release,
      assets: release.assets.map(asset => asset.name === "SHA256SUMS.txt"
        ? { ...asset, size: Buffer.byteLength(checksums), digest: `sha256:${createHash("sha256").update(checksums).digest("hex")}` }
        : asset),
    };
    const selected = selectDeveloperNightly([actual], CAD_REPO, "cadcraft");
    assert.ok(selected, "the published metadata still advertises a portable archive");
    await generateLauncherCatalog({
      baseCatalogPath, outCatalogPath,
      targets: [{
        id: "cadcraft", repo: CAD_REPO, channel: "nightly", tag: selected.tag, version: selected.version,
        prerelease: true, required: false, publishedAt: selected.publishedAt, publishedAssets: selected.assets,
      }],
      fetchImpl: async () => ({ ok: true, status: 200, arrayBuffer: async () => Buffer.from(checksums) }),
    });
    const py = spawnSync("python3", ["-c",
      'import json, sys, tomllib; print(json.dumps(tomllib.load(open(sys.argv[1], "rb"))["apps"][0]["releases"]["nightly"]))',
      outCatalogPath], { encoding: "utf8" });
    assert.equal(py.status, 0, py.stderr);
    return JSON.parse(py.stdout);
  }
  const single = await generate(sums);
  assert.equal(single.published_at, CAD_PUBLISHED);
  assert.ok(single.assets.some(asset => asset.name.endsWith("windows-x86_64-nightly.zip")));
  const descriptorOnly = await generate(`${HASH_A}  NIGHTLY.json\n`);
  assert.equal(descriptorOnly.published_at, undefined);
  assert.equal(descriptorOnly.assets, undefined);
});
