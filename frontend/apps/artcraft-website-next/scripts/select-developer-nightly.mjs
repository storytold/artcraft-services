// Select the highest immutable developer build from published GitHub release metadata.
// Keep this distinct from Linux repository metadata, which may have older package-only releases.
const TAG = /^v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)-nightly\.([1-9][0-9]{7})\.([1-9][0-9]*)\.([1-9][0-9]*)\.g([0-9a-f]{12})$/;
const PAYLOAD_SUFFIXES = [
  "linux-x86_64-nightly.tar.gz",
  "linux-aarch64-nightly.tar.gz",
  "macos-aarch64-nightly.zip",
  "macos-x86_64-nightly.zip",
  "windows-x86_64-nightly.zip",
  "windows-aarch64-nightly.zip",
];

export function hasPortablePayload(names, slug, version) {
  return PAYLOAD_SUFFIXES.some(suffix => names.has(`${slug}-${version}-${suffix}`));
}

function compare(a, b) {
  for (let i = 1; i <= 6; i += 1) {
    const x = BigInt(a[i]), y = BigInt(b[i]);
    if (x !== y) return x > y ? 1 : -1;
  }
  return a[7] < b[7] ? -1 : a[7] > b[7] ? 1 : 0;
}

export function selectDeveloperNightly(releases, repo, slug) {
  let best = null;
  const seen = new Set();
  for (const release of releases) {
    if (release.draft !== false || release.prerelease !== true) continue;
    const tag = release.tag_name;
    if (typeof tag !== "string" || tag.length > 64) continue;
    const parsed = TAG.exec(tag);
    if (!parsed) continue;
    if (seen.has(tag)) return null; // Incomplete pagination; do not publish a winning claim.
    seen.add(tag);
    // Compare canonical SemVer components before the lexical tag tie-break (not publication time).
    const order = best ? compare(parsed, best.parsed) : 1;
    if (order > 0 || (order === 0 && tag < best.release.tag_name)) best = { parsed, release };
  }
  if (!best) return null;

  // Verify metadata availability for the winner only; if it is incomplete, don't silently
  // select a lower release. Let the client use its complete existing GitHub lookup.
  const release = best.release;
  const tag = release.tag_name;
  const publishedAt = release.published_at;
  if (typeof publishedAt !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/.test(publishedAt) || !Number.isFinite(Date.parse(publishedAt))) return null;
  const version = tag.replace(/^v/, "");
  const assets = [];
  const names = new Set();
  for (const asset of release.assets ?? []) {
    if (
      typeof asset.name !== "string" ||
      names.has(asset.name) ||
      asset.state !== "uploaded" ||
      !Number.isSafeInteger(asset.size) ||
      asset.size <= 0 ||
      asset.browser_download_url !== `https://github.com/${repo}/releases/download/${tag}/${asset.name}`
    ) return null;
    names.add(asset.name);
    assets.push({
      name: asset.name,
      size: asset.size,
      ...(/^sha256:[a-fA-F0-9]{64}$/.test(asset.digest ?? "") ? { sha256: asset.digest.slice(7).toLowerCase() } : {}),
    });
  }
  if (!names.has("NIGHTLY.json") || !names.has("SHA256SUMS.txt") ||
      !hasPortablePayload(names, slug, version)) return null;
  return { tag, version, publishedAt, assets: assets.sort((a, b) => a.name.localeCompare(b.name)) };
}
