import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SOURCE = "https://github.com/storytold/craft-launcher-assets.git";
const ICON = /^[a-z0-9-]+-([a-f0-9]{64})\.png$/;
const NOTICE = /^(?:ATTRIBUTION\.md|LICENSE-[A-Za-z0-9-]+\.txt)$/;
const SCREENSHOT = /^[a-z0-9-]+\/[a-z0-9-]+\.jpg$/;
const VERSION = /^v[1-9]\d*$/;

function git(repo, args, maxBuffer = 4 << 20) {
  return execFileSync("git", ["-C", repo, ...args], { maxBuffer });
}

// Read Git blobs, not a working-tree checkout: only ordinary files in the documented
// asset paths can reach out/, and nothing in the asset repository is executed.
function files(repo, commit) {
  return git(repo, ["ls-tree", "-rz", "--full-tree", commit]).toString("utf8")
    .split("\0").filter(Boolean).map((entry) => {
      const tab = entry.indexOf("\t");
      const [mode, type, oid] = entry.slice(0, tab).split(" ");
      return { mode, type, oid, path: entry.slice(tab + 1) };
    });
}

function readBlob(repo, entry, max) {
  if (!entry || entry.type !== "blob" || entry.mode !== "100644") {
    throw new Error("Expected an ordinary Git asset file: " + (entry?.path ?? "missing"));
  }
  const bytes = git(repo, ["cat-file", "blob", entry.oid], max);
  if (bytes.length > max) throw new Error("Asset exceeds byte limit: " + entry.path);
  return bytes;
}

function output(outDir, rel, bytes) {
  const path = join(outDir, rel);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, bytes);
}

export function copyLauncherAssets({ repo, outDir, revision = "refs/heads/main" }) {
  const commit = git(repo, ["rev-parse", "--verify", revision + "^{commit}"]).toString().trim();
  const tree = files(repo, commit);
  const catalog = readBlob(repo, tree.find((e) => e.path === "catalog.toml"), 1 << 20).toString("utf8");
  // Replace generated outputs only, so a prior local sync cannot leak old assets into a deploy.
  for (const name of ["launcher-app-icons", "launcher-screenshots"]) {
    rmSync(join(outDir, name), { recursive: true, force: true });
  }
  const icons = new Set();
  for (const entry of tree.filter((e) => e.path.startsWith("app-icons/"))) {
    const name = entry.path.slice("app-icons/".length);
    const icon = ICON.exec(name);
    if (!icon && !NOTICE.test(name)) continue;
    const bytes = readBlob(repo, entry, icon ? 256 << 10 : 1 << 20);
    if (icon && createHash("sha256").update(bytes).digest("hex") !== icon[1]) {
      throw new Error("Icon digest does not match its filename: " + name);
    }
    output(outDir, "launcher-app-icons/" + name, bytes);
    if (icon) icons.add(name);
  }
  // The base catalog uses simple quoted icon URLs and digests. Require every referenced
  // file to be present and content-addressed before the enrichment/signing step.
  const references = [...catalog.matchAll(/^\s*icon_url\s*=\s*"https:\/\/getartcraft\.com\/launcher-app-icons\/([^"\r\n]+)"\s*$/gm)];
  const digests = [...catalog.matchAll(/^\s*icon_sha256\s*=\s*"([a-f0-9]{64})"\s*$/gm)];
  const appCount = [...catalog.matchAll(/^\s*\[\[apps\]\]\s*$/gm)].length;
  if (!appCount || references.length !== appCount || digests.length !== appCount ||
      references.some((r, i) => !icons.has(r[1]) || ICON.exec(r[1])?.[1] !== digests[i][1])) {
    throw new Error("Catalog icon references do not match the fetched assets");
  }
  output(outDir, "launcher-catalog.toml", "# Assets revision: " + commit + "\n" + catalog);

  // Screenshots have immutable, versioned public URLs. Keep every published version,
  // rather than rewriting v1 from the moving main branch.
  const tags = git(repo, ["tag", "--list", "v*"]).toString().split("\n").filter((tag) => VERSION.test(tag));
  if (!tags.includes("v1")) throw new Error("Published screenshot version v1 is missing");
  for (const tag of tags) {
    const taggedCommit = git(repo, ["rev-parse", "--verify", "refs/tags/" + tag + "^{commit}"]).toString().trim();
    const taggedFiles = files(repo, taggedCommit);
    for (const entry of taggedFiles.filter((e) => SCREENSHOT.test(e.path))) {
      output(outDir, "launcher-screenshots/" + tag + "/" + entry.path, readBlob(repo, entry, 2 << 20));
    }
    const attribution = taggedFiles.find((e) => e.path === "ATTRIBUTION.md");
    if (attribution) output(outDir, "launcher-screenshots/" + tag + "/ATTRIBUTION.md", readBlob(repo, attribution, 1 << 20));
  }
  console.info("Launcher assets:", commit, "(" + icons.size + " icons; screenshot tags " + tags.join(", ") + ")");
  return { commit, icons: icons.size, tags };
}

export function fetchLauncherAssets(outDir) {
  const dir = mkdtempSync(join(tmpdir(), "launcher-assets-"));
  try {
    execFileSync("git", ["clone", "--bare", SOURCE, join(dir, "repo")], { stdio: "inherit" });
    return copyLauncherAssets({ repo: join(dir, "repo"), outDir });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  fetchLauncherAssets(resolve(process.argv[2] ?? "public"));
}
