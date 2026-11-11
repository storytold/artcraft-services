import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, renameSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { signLauncherCatalog } from "./sign-launcher-catalog.mjs";

const ROOT = process.cwd();
const APP_DIR = join(ROOT, "src", "app");
const STASH_DIR = join(ROOT, ".build-route-stash");

// Dynamic route handlers that Next.js dev/Playwright tests use directly,
// but that are replaced in production by build-time static files + the Cloudflare Worker.
const DYNAMIC_ROUTE_PATHS = [
  "apt",
  "rpm",
  "arch",
  "keys",
  "index",
  "oci-signatures",
  "artcraft-oci.flatpakrepo",
  "apps/[slug]/[filename]",
  "media/[token]/oembed",
  "media/[token]/player",
];

/**
 * Temporarily moves dynamic Next.js route handlers out of `src/app` before running
 * a static export build, recovering any routes left behind by an interrupted build first.
 */
function stashRoutes() {
  restoreRoutes();
  rmSync(STASH_DIR, { recursive: true, force: true });
  for (const rel of DYNAMIC_ROUTE_PATHS) {
    const src = join(APP_DIR, rel);
    if (!existsSync(src)) continue;
    const dest = join(STASH_DIR, rel);
    mkdirSync(dirname(dest), { recursive: true });
    renameSync(src, dest);
  }
}

/**
 * Restores stashed dynamic Next.js route handlers back into `src/app` and removes the stash directory.
 */
function restoreRoutes() {
  if (!existsSync(STASH_DIR)) return;
  for (const rel of DYNAMIC_ROUTE_PATHS) {
    const stashed = join(STASH_DIR, rel);
    if (!existsSync(stashed)) continue;
    const target = join(APP_DIR, rel);
    mkdirSync(dirname(target), { recursive: true });
    renameSync(stashed, target);
  }
  rmSync(STASH_DIR, { recursive: true, force: true });
}

for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => {
    restoreRoutes();
    process.exit(1);
  });
}

try {
  stashRoutes();
  const nextBin = join(ROOT, "node_modules", ".bin", "next");
  execFileSync(nextBin, ["build"], {
    stdio: "inherit",
    env: { ...process.env, NEXT_STATIC_EXPORT: "1" },
  });
} finally {
  restoreRoutes();
}

const jitiBin = join(ROOT, "node_modules", ".bin", "jiti");
execFileSync(process.execPath, ["--no-warnings", jitiBin, join(ROOT, "scripts", "generate-static-repos.mjs")], {
  stdio: "inherit",
  env: process.env,
});

// Sign the final exported bytes; missing/invalid secrets fail the deployment build.
signLauncherCatalog(join(ROOT, "out"));
