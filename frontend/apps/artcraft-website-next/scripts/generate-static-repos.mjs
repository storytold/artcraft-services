import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { gzipSync } from "node:zlib";
import { artcraftStudioDownloads } from "../src/lib/artcraft-studio.ts";
import { getTutorialItems } from "../src/lib/content.ts";
import { craftLauncherDownloads } from "../src/lib/craft-launcher.ts";
import {
  CRAFTING_APPS,
  craftAppName,
  craftAppRelease,
  craftReleaseDownloads,
} from "../src/lib/crafting-apps.ts";
import { getFlatpakOciIndex } from "../src/lib/flatpak-oci.ts";
import { buildOciSignaturePayload } from "../src/lib/flatpak-signatures.ts";
import {
  RPM_REPO_TIMESTAMP,
  VALID_APT_ARCHES,
  VALID_PACMAN_ARCHES,
  VALID_RPM_ARCHES,
  generateAptPackages,
  generateAptRelease,
  generatePacmanDb,
  generateRpmRepomdXml,
  getLinuxRepoApps,
  getRpmPrimaryXml,
  resolvePackageDownloadUrl,
} from "../src/lib/linux-repos.ts";
import {
  getSigningPublicKey,
  isRepoSigningConfigured,
  signCleartext,
  signDetachedArmored,
  signDetachedBinary,
  signInlineBinary,
} from "../src/lib/repo-signing.ts";

const ROOT = process.cwd();
const OUT_DIR = join(ROOT, "out");
const WORKER_DIR = join(ROOT, "worker");

/**
 * Writes a generated repository artifact under the static `out/` directory.
 */
function writeOut(relPath, content) {
  const fullPath = join(OUT_DIR, relPath);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, content);
}

console.info("Generating static Linux repositories and Flatpak OCI catalog...");

const [signingConfigured, publicKey, , flatpakIndex] = await Promise.all([
  isRepoSigningConfigured(),
  getSigningPublicKey(),
  getLinuxRepoApps({ deb: true, rpm: true, arch: true }),
  getFlatpakOciIndex(),
]);

if (signingConfigured && !publicKey) {
  throw new Error("Repo signing is configured but the public key could not be loaded");
}
if (signingConfigured && publicKey) {
  writeOut("keys/artcraft.asc", publicKey.armored);
  writeOut("keys/artcraft.gpg", Buffer.from(publicKey.binary));
}

/**
 * Generates APT `Packages`, `Packages.gz`, `Release`, and optional `Release.gpg`/`InRelease` signatures.
 */
async function buildAptRepo() {
  await Promise.all(
    [...VALID_APT_ARCHES].map(async (arch) => {
      const packages = await generateAptPackages(arch);
      const base = `apt/dists/stable/main/binary-${arch}`;
      writeOut(`${base}/Packages`, packages);
      writeOut(`${base}/Packages.gz`, gzipSync(Buffer.from(packages, "utf-8")));
    }),
  );

  const aptRelease = await generateAptRelease();
  writeOut("apt/dists/stable/Release", aptRelease);
  if (signingConfigured) {
    const [releaseGpg, inRelease] = await Promise.all([
      signDetachedArmored(aptRelease),
      signCleartext(aptRelease),
    ]);
    if (!releaseGpg || !inRelease) {
      throw new Error("Failed to sign the apt Release file");
    }
    writeOut("apt/dists/stable/Release.gpg", releaseGpg);
    writeOut("apt/dists/stable/InRelease", inRelease);
  }
}

/**
 * Generates RPM `primary.xml.gz`, `repomd.xml`, and optional `repomd.xml.asc` for a repository prefix.
 */
async function buildRpmTarget(prefix, archFilter) {
  const primaryXml = await getRpmPrimaryXml(archFilter);
  const primaryBuffer = Buffer.from(primaryXml, "utf-8");
  const primaryGzBuffer = gzipSync(primaryBuffer);
  const repomd = generateRpmRepomdXml(primaryBuffer, primaryGzBuffer, RPM_REPO_TIMESTAMP);

  writeOut(`${prefix}/repodata/primary.xml.gz`, primaryGzBuffer);
  writeOut(`${prefix}/repodata/repomd.xml`, repomd);
  if (signingConfigured) {
    const repomdAsc = await signDetachedArmored(repomd);
    if (!repomdAsc) {
      throw new Error(`Failed to sign ${prefix}/repodata/repomd.xml`);
    }
    writeOut(`${prefix}/repodata/repomd.xml.asc`, repomdAsc);
  }
}

/**
 * Generates top-level and architecture-specific RPM repositories and public key files.
 */
async function buildRpmRepos() {
  if (signingConfigured && publicKey) {
    writeOut("rpm/repodata/repomd.xml.key", publicKey.armored);
  }
  await Promise.all([
    buildRpmTarget("rpm", undefined),
    ...[...VALID_RPM_ARCHES].map((arch) => buildRpmTarget(`rpm/${arch}`, arch)),
  ]);
}

/**
 * Generates Arch Linux Pacman `artcraft.db`, `artcraft.db.tar.gz`, and optional `artcraft.db.sig` files.
 */
async function buildPacmanRepos() {
  await Promise.all(
    [...VALID_PACMAN_ARCHES].map(async (arch) => {
      const db = await generatePacmanDb(arch);
      writeOut(`arch/${arch}/artcraft.db`, db);
      writeOut(`arch/${arch}/artcraft.db.tar.gz`, db);
      if (signingConfigured) {
        const sig = await signDetachedBinary(new Uint8Array(db));
        if (!sig) {
          throw new Error(`Failed to sign arch/${arch}/artcraft.db`);
        }
        writeOut(`arch/${arch}/artcraft.db.sig`, Buffer.from(sig));
      }
    }),
  );
}

/**
 * Generates Flatpak OCI lookaside OpenPGP signature packets (`signature-1`) for every indexed image digest.
 */
async function buildFlatpakSignatures() {
  if (!signingConfigured) return;
  const tasks = [];
  for (const repo of flatpakIndex.Results) {
    const seenDigests = new Set();
    for (const image of repo.Images) {
      if (!image.Digest.startsWith("sha256:") || seenDigests.has(image.Digest)) continue;
      seenDigests.add(image.Digest);
      tasks.push(
        (async () => {
          const payload = buildOciSignaturePayload(image.Digest, repo.Repository);
          const sig = await signInlineBinary(Buffer.from(payload, "utf-8"));
          if (!sig) {
            throw new Error(`Failed to sign Flatpak OCI digest ${image.Digest} for ${repo.Repository}`);
          }
          const hex = image.Digest.slice("sha256:".length);
          writeOut(`oci-signatures/${repo.Repository}@sha256=${hex}/signature-1`, Buffer.from(sig));
        })(),
      );
    }
  }
  await Promise.all(tasks);
}

await Promise.all([
  buildAptRepo(),
  buildRpmRepos(),
  buildPacmanRepos(),
  buildFlatpakSignatures(),
]);

// Build static lookup tables for the Cloudflare Worker
const packageRedirects = {};
const allDownloads = [
  ...artcraftStudioDownloads(),
  ...craftLauncherDownloads(),
  ...CRAFTING_APPS.flatMap((app) => {
    const release = craftAppRelease(app);
    return release ? craftReleaseDownloads(app, release) : [];
  }),
];

for (const item of allDownloads) {
  const resolved = resolvePackageDownloadUrl(item.fileName);
  if (resolved) {
    packageRedirects[item.fileName] = resolved;
  }
  if (item.fileName.endsWith(".rpm")) {
    const ascName = `${item.fileName}.asc`;
    const ascUrl = resolvePackageDownloadUrl(ascName);
    if (ascUrl) packageRedirects[ascName] = ascUrl;
  }
  if (item.fileName.endsWith(".pkg.tar.zst")) {
    const sigName = `${item.fileName}.sig`;
    const sigUrl = resolvePackageDownloadUrl(sigName);
    if (sigUrl) packageRedirects[sigName] = sigUrl;
  }
}

const flatpakApps = Object.fromEntries(
  CRAFTING_APPS.map((app) => [
    app.slug,
    {
      title: craftAppName(app),
      appId: `ai.storyteller.${app.slug}`,
    },
  ]),
);

const tutorialRedirects = {};
for (const item of getTutorialItems()) {
  for (const alias of item.aliases) {
    if (alias !== item.slug) {
      tutorialRedirects[alias] = item.slug;
    }
  }
}

mkdirSync(WORKER_DIR, { recursive: true });
writeFileSync(
  join(WORKER_DIR, "generated-catalog.mjs"),
  `// Generated at build time by scripts/generate-static-repos.mjs — do not edit by hand.
export const SIGNING_CONFIGURED = ${JSON.stringify(signingConfigured)};
export const PUBLIC_KEY_BASE64 = ${JSON.stringify(
    publicKey ? Buffer.from(publicKey.binary).toString("base64") : null,
  )};
export const FLATPAK_APPS = ${JSON.stringify(flatpakApps)};
export const FLATPAK_INDEX = ${JSON.stringify(flatpakIndex)};
export const PACKAGE_REDIRECTS = ${JSON.stringify(packageRedirects)};
export const TUTORIAL_REDIRECTS = ${JSON.stringify(tutorialRedirects)};
`,
);

console.info(
  `Static repository generation complete (signed: ${signingConfigured}, OCI repos: ${flatpakIndex.Results.length}, package redirects: ${Object.keys(packageRedirects).length}).`,
);
