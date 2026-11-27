import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Matches craft-launcher's MAX_CATALOG_BYTES (1 MiB) so deployed catalogs never exceed the
// launcher's verification limit.
export const MAX_CATALOG_BYTES = 1 << 20;

const GITHUB_BASE = "https://github.com";
const MAX_FETCH_ATTEMPTS = 3;
const FETCH_TIMEOUT_MS = 10_000;
const FETCH_CONCURRENCY = 6;

const INVALID_FILENAME_CHARS = /[\x00-\x1f\x7f/\\:<>"|?*]/;
const HEX64 = /^[0-9a-f]{64}$/i;

/**
 * Validates a release asset filename using the same rules as ArtCraft Launcher.
 */
export function isValidFileName(name) {
  return (
    typeof name === "string" &&
    name.length > 0 &&
    name.length <= 200 &&
    !name.startsWith(".") &&
    !INVALID_FILENAME_CHARS.test(name)
  );
}

/**
 * Validates a Git tag before interpolating it into a GitHub release URL.
 */
export function isValidTag(tag) {
  return (
    typeof tag === "string" &&
    tag.length > 0 &&
    tag.length <= 64 &&
    !/[\s/\\?#\x00-\x1f\x7f]/.test(tag)
  );
}

/**
 * Parses a `SHA256SUMS.txt` file into sorted `{ name, sha256 }` entries.
 * Invalid lines, malformed hashes, and unsafe filenames are skipped.
 */
export function parseSha256Sums(text) {
  const byName = new Map();
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    const match = line.match(/^([0-9a-fA-F]{64})\s+\*?(.+)$/);
    if (!match) continue;
    const sha256 = match[1].toLowerCase();
    const name = match[2].trim();
    if (!HEX64.test(sha256) || !isValidFileName(name)) continue;
    byName.set(name, sha256);
  }
  return [...byName.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, sha256]) => ({ name, sha256 }));
}

/**
 * Escapes a string as a TOML basic string literal.
 */
export function formatTomlString(value) {
  let out = '"';
  for (const ch of String(value)) {
    const code = ch.codePointAt(0);
    if (ch === "\\") out += "\\\\";
    else if (ch === '"') out += '\\"';
    else if (ch === "\b") out += "\\b";
    else if (ch === "\t") out += "\\t";
    else if (ch === "\n") out += "\\n";
    else if (ch === "\f") out += "\\f";
    else if (ch === "\r") out += "\\r";
    else if (code < 0x20 || code === 0x7f) {
      out += `\\u${code.toString(16).padStart(4, "0")}`;
    } else {
      out += ch;
    }
  }
  return `${out}"`;
}

/**
 * Fetches `SHA256SUMS.txt` for a GitHub release tag with retries.
 * Returns null if `SHA256SUMS.txt` is not published (HTTP 404).
 */
export async function fetchReleaseAssets(repo, tag, fetchImpl = fetch) {
  if (!isValidTag(tag)) {
    throw new Error(`Invalid release tag "${tag}" for ${repo}`);
  }
  const sumsUrl = `${GITHUB_BASE}/${repo}/releases/download/${encodeURIComponent(tag)}/SHA256SUMS.txt`;
  let lastError = new Error(`Unable to fetch ${sumsUrl}`);

  for (let attempt = 1; attempt <= MAX_FETCH_ATTEMPTS; attempt += 1) {
    try {
      const res = await fetchImpl(sumsUrl, {
        signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
      });
      if (res.status === 404) {
        return null;
      }
      if (res.ok) {
        const text = await res.text();
        const entries = parseSha256Sums(text);
        return entries.map(({ name, sha256 }) => ({
          name,
          download_url: `${GITHUB_BASE}/${repo}/releases/download/${tag}/${name}`,
          sha256,
        }));
      }
      lastError = new Error(`HTTP ${res.status} fetching ${sumsUrl}`);
    } catch (error) {
      lastError = error instanceof Error ? error : lastError;
    }
    if (attempt < MAX_FETCH_ATTEMPTS) {
      await new Promise((r) => setTimeout(r, 500 * attempt));
    }
  }
  throw lastError;
}

/**
 * Formats a single release channel (`stable`, `prerelease`, or `nightly`) under `tablePrefix`.
 */
export function formatReleaseSection(tablePrefix, release) {
  const lines = [
    `[${tablePrefix}]`,
    `tag = ${formatTomlString(release.tag)}`,
    `version = ${formatTomlString(release.version)}`,
    `html_url = ${formatTomlString(release.html_url)}`,
    `prerelease = ${release.prerelease ? "true" : "false"}`,
  ];
  for (const asset of release.assets ?? []) {
    lines.push(`[[${tablePrefix}.assets]]`);
    lines.push(`name = ${formatTomlString(asset.name)}`);
    lines.push(`download_url = ${formatTomlString(asset.download_url)}`);
    if (asset.sha256) {
      lines.push(`sha256 = ${formatTomlString(asset.sha256)}`);
    }
  }
  return lines.join("\n");
}

function formatChannelsBlock(prefix, channels) {
  if (!channels) return "";
  const sections = [];
  for (const channel of ["stable", "prerelease", "nightly"]) {
    if (channels[channel]) {
      sections.push(formatReleaseSection(`${prefix}.${channel}`, channels[channel]));
    }
  }
  return sections.join("\n");
}

/**
 * Enriches the committed `public/launcher-catalog.toml` text with statically generated
 * `generated_at`, `[launcher]` release channels, and per-app `[apps.releases.*]` tables.
 * Preserves all existing `[[apps]]` entries and display order.
 */
export function enrichLauncherCatalog(baseToml, { generatedAt, launcherReleases, appReleasesById }) {
  const firstAppMatch = baseToml.match(/(^|\n)\[\[apps\]\]\r?\n/);
  if (!firstAppMatch) {
    throw new Error("Base launcher-catalog.toml contains no [[apps]] entries");
  }

  const splitIndex = firstAppMatch.index + (firstAppMatch[1] ? firstAppMatch[1].length : 0);
  const headerPart = baseToml.slice(0, splitIndex).trimEnd();
  const appsPart = baseToml.slice(splitIndex);

  const preambleLines = [headerPart, "", `generated_at = ${formatTomlString(generatedAt)}`];

  const launcherSections = formatChannelsBlock("launcher.releases", launcherReleases);
  if (launcherSections) {
    preambleLines.push(
      "",
      "[launcher]",
      `id = ${formatTomlString("artcraft-launcher")}`,
      `name = ${formatTomlString("ArtCraft Launcher")}`,
      `repo = ${formatTomlString("storytold/craft-launcher")}`,
      launcherSections,
    );
  }

  // Split into individual `[[apps]]` blocks so each block's `[apps.releases.*]` attaches to it.
  const appBlocks = appsPart.split(/(?=^\[\[apps\]\]\r?\n)/m).filter((block) => block.trim().length > 0);
  const enrichedBlocks = appBlocks.map((block) => {
    const trimmed = block.trimEnd();
    const idMatch = trimmed.match(/^id\s*=\s*"([^"]+)"/m);
    if (!idMatch) {
      return trimmed;
    }
    const appId = idMatch[1];
    const channelsText = formatChannelsBlock("apps.releases", appReleasesById?.[appId]);
    return channelsText ? `${trimmed}\n${channelsText}` : trimmed;
  });

  const output = `${preambleLines.join("\n")}\n\n${enrichedBlocks.join("\n\n")}\n`;
  const byteLength = Buffer.byteLength(output, "utf8");
  if (byteLength > MAX_CATALOG_BYTES) {
    throw new Error(
      `Generated launcher-catalog.toml is ${byteLength} bytes, exceeding MAX_CATALOG_BYTES (${MAX_CATALOG_BYTES})`,
    );
  }
  return output;
}

/**
 * Runs async tasks with bounded concurrency.
 */
async function mapWithConcurrency(items, limit, fn) {
  const results = new Array(items.length);
  let nextIndex = 0;

  async function worker() {
    while (nextIndex < items.length) {
      const index = nextIndex;
      nextIndex += 1;
      results[index] = await fn(items[index], index);
    }
  }

  const workers = Array.from({ length: Math.min(limit, items.length) }, () => worker());
  await Promise.all(workers);
  return results;
}

/**
 * Collects all configured release targets from `crafting-app-releases.ts`.
 */
export async function collectConfiguredReleases() {
  const {
    CRAFT_APP_RELEASES,
    CRAFT_COMMUNITY_RELEASES,
    CRAFT_LAUNCHER_RELEASE,
    CRAFT_NIGHTLY_RELEASES,
    CRAFT_PRERELEASE_RELEASES,
  } = await import("../src/lib/crafting-app-releases.ts");

  const targets = [];

  if (CRAFT_LAUNCHER_RELEASE) {
    const tag = CRAFT_LAUNCHER_RELEASE.tag ?? `v${CRAFT_LAUNCHER_RELEASE.version}`;
    targets.push({
      id: "artcraft-launcher",
      repo: "storytold/craft-launcher",
      channel: "stable",
      tag,
      version: CRAFT_LAUNCHER_RELEASE.version,
      prerelease: false,
      required: true,
    });
  }
  if (CRAFT_PRERELEASE_RELEASES["artcraft-launcher"]) {
    const pre = CRAFT_PRERELEASE_RELEASES["artcraft-launcher"];
    targets.push({
      id: "artcraft-launcher",
      repo: "storytold/craft-launcher",
      channel: "prerelease",
      tag: pre.tag,
      version: pre.version,
      prerelease: true,
      required: false,
    });
  }
  if (CRAFT_NIGHTLY_RELEASES["artcraft-launcher"]) {
    const nightly = CRAFT_NIGHTLY_RELEASES["artcraft-launcher"];
    targets.push({
      id: "artcraft-launcher",
      repo: "storytold/craft-launcher",
      channel: "nightly",
      tag: nightly.tag,
      version: nightly.version,
      prerelease: true,
      required: false,
    });
  }

  for (const [slug, release] of Object.entries(CRAFT_APP_RELEASES)) {
    const repo = `storytold/${slug}`;
    if (release) {
      const tag = release.tag ?? `v${release.version}`;
      targets.push({
        id: slug,
        repo,
        channel: "stable",
        tag,
        version: release.version,
        prerelease: false,
        required: true,
      });
    }
    const pre = CRAFT_PRERELEASE_RELEASES[slug];
    if (pre) {
      targets.push({
        id: slug,
        repo,
        channel: "prerelease",
        tag: pre.tag,
        version: pre.version,
        prerelease: true,
        required: false,
      });
    }
    const nightly = CRAFT_NIGHTLY_RELEASES[slug];
    if (nightly) {
      targets.push({
        id: slug,
        repo,
        channel: "nightly",
        tag: nightly.tag,
        version: nightly.version,
        prerelease: true,
        required: false,
      });
    }
  }

  for (const [appId, entry] of Object.entries(CRAFT_COMMUNITY_RELEASES)) {
    for (const channel of ["stable", "prerelease", "nightly"]) {
      const ch = entry[channel];
      if (!ch) continue;
      targets.push({
        id: appId,
        repo: entry.repo,
        channel,
        tag: ch.tag,
        version: ch.version,
        prerelease: channel !== "stable",
        required: false,
      });
    }
  }

  return targets;
}

/**
 * Generates the static `out/launcher-catalog.toml` from `public/launcher-catalog.toml`
 * and the configured release versions + GitHub `SHA256SUMS.txt` checksums.
 */
export async function generateLauncherCatalog({
  baseCatalogPath,
  outCatalogPath,
  generatedAt = new Date().toISOString(),
  targets,
  fetchImpl = fetch,
}) {
  const resolvedTargets = targets ?? (await collectConfiguredReleases());
  const baseToml = readFileSync(baseCatalogPath, "utf8");
  const launcherReleases = {};
  const appReleasesById = {};

  await mapWithConcurrency(resolvedTargets, FETCH_CONCURRENCY, async (target) => {
    let assets = [];
    try {
      const fetched = await fetchReleaseAssets(target.repo, target.tag, fetchImpl);
      if (fetched) {
        assets = fetched;
      } else if (target.required) {
        throw new Error(`SHA256SUMS.txt not found for ${target.repo}@${target.tag}`);
      }
    } catch (error) {
      if (target.required) {
        throw error;
      }
      console.warn(
        `Warning: could not fetch SHA256SUMS.txt for ${target.repo}@${target.tag} (${target.channel}): ${error instanceof Error ? error.message : error}`,
      );
    }

    const record = {
      tag: target.tag,
      version: target.version,
      html_url: `${GITHUB_BASE}/${target.repo}/releases/tag/${target.tag}`,
      prerelease: target.prerelease,
      assets,
    };

    if (target.id === "artcraft-launcher") {
      launcherReleases[target.channel] = record;
    } else {
      if (!appReleasesById[target.id]) {
        appReleasesById[target.id] = {};
      }
      appReleasesById[target.id][target.channel] = record;
    }
  });

  const output = enrichLauncherCatalog(baseToml, {
    generatedAt,
    launcherReleases,
    appReleasesById,
  });

  writeFileSync(outCatalogPath, output, "utf8");
  return {
    bytes: Buffer.byteLength(output, "utf8"),
    releaseCount: resolvedTargets.length,
  };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const ROOT = process.cwd();
  const baseCatalogPath = join(ROOT, "public", "launcher-catalog.toml");
  const outCatalogPath = join(ROOT, "out", "launcher-catalog.toml");
  console.info("Generating static launcher-catalog.toml release metadata...");
  const { bytes, releaseCount } = await generateLauncherCatalog({
    baseCatalogPath,
    outCatalogPath,
  });
  console.info(`Generated out/launcher-catalog.toml (${releaseCount} release channels, ${bytes} bytes).`);
}
