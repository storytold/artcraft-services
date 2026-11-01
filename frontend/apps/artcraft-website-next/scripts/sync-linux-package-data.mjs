#!/usr/bin/env node
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { execSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageDataJsonPath = join(__dirname, "../src/lib/linux-package-data.json");
const releasesModulePath = join(__dirname, "../src/lib/crafting-app-releases.ts");

const args = process.argv.slice(2);
const isCheckOnly = args.includes("--check");
const isForce = args.includes("--force");

async function main() {
  const startTime = Date.now();

  const { CRAFT_LAUNCHER_RELEASE, CRAFT_ANYTHING_RELEASE, CRAFT_APP_RELEASES } =
    await import(releasesModulePath);

  let existingData = {};
  if (existsSync(packageDataJsonPath)) {
    try {
      existingData = JSON.parse(readFileSync(packageDataJsonPath, "utf-8"));
    } catch {
      existingData = {};
    }
  }

  const apps = [];

  if (CRAFT_LAUNCHER_RELEASE) {
    apps.push({
      repo: "craft-launcher",
      name: "artcraft-launcher",
      version: CRAFT_LAUNCHER_RELEASE.version,
      debArches: ["x86_64", "aarch64", "riscv64"],
      rpmArches: ["x86_64", "aarch64", "riscv64"],
      archArches: ["x86_64", "aarch64"],
    });
  }

  if (CRAFT_ANYTHING_RELEASE) {
    apps.push({
      repo: "craft-anything",
      name: "craft-anything",
      version: CRAFT_ANYTHING_RELEASE.version,
      debArches: ["x86_64", "aarch64"],
      rpmArches: ["x86_64", "aarch64"],
      archArches: [],
    });
  }

  for (const [slug, release] of Object.entries(CRAFT_APP_RELEASES)) {
    if (!release) continue;
    apps.push({
      repo: slug,
      name: slug,
      version: release.version,
      debArches: ["x86_64", "aarch64", "riscv64"],
      rpmArches: ["x86_64", "aarch64", "riscv64"],
      archArches: ["x86_64", "aarch64"],
    });
  }

  // Build required filenames list
  const requiredFiles = [];
  for (const app of apps) {
    for (const arch of app.debArches) {
      requiredFiles.push({
        filename: `${app.name}-${app.version}-linux-${arch}.deb`,
        app,
        type: "deb",
      });
    }
    for (const arch of app.rpmArches) {
      requiredFiles.push({
        filename: `${app.name}-${app.version}-linux-${arch}.rpm`,
        app,
        type: "rpm",
      });
    }
    for (const arch of app.archArches) {
      requiredFiles.push({
        filename: `${app.name}-${app.version}-linux-${arch}.pkg.tar.zst`,
        app,
        type: "arch",
      });
    }
  }

  if (isCheckOnly) {
    const missing = requiredFiles.filter(
      (item) => !existingData[item.filename] || !existingData[item.filename].sha256
    );
    if (missing.length > 0) {
      console.error(
        `Error: ${missing.length} package(s) missing from linux-package-data.json:`
      );
      for (const item of missing) {
        console.error(`  - ${item.filename}`);
      }
      console.error("Run `npm run sync:packages` to synchronize package data.");
      process.exit(1);
    }
    console.log(
      `✓ All ${requiredFiles.length} required package entries verified in linux-package-data.json.`
    );
    return;
  }

  // Identify apps that have missing or changed files
  const appsToFetch = apps.filter((app) => {
    if (isForce) return true;
    const appFiles = requiredFiles.filter((item) => item.app === app);
    return appFiles.some(
      (item) => !existingData[item.filename] || !existingData[item.filename].sha256
    );
  });

  const nextData = { ...existingData };

  if (appsToFetch.length > 0) {
    console.log(
      `Synchronizing package metadata for ${appsToFetch.length} app(s) from GitHub releases...`
    );

    await Promise.all(
      appsToFetch.map(async (app) => {
        const sumsUrl = `https://github.com/storytold/${app.repo}/releases/download/v${app.version}/SHA256SUMS.txt`;
        try {
          const sumsRes = await fetch(sumsUrl);
          if (!sumsRes.ok) {
            console.warn(
              `Warning: Could not fetch ${sumsUrl} (${sumsRes.status} ${sumsRes.statusText})`
            );
            return;
          }
          const sumsText = await sumsRes.text();
          const shaMap = {};
          for (const line of sumsText.split("\n")) {
            const parts = line.trim().split(/\s+/);
            if (parts.length >= 2) {
              shaMap[parts[1]] = parts[0];
            }
          }

          const appFiles = requiredFiles.filter((item) => item.app === app);
          await Promise.all(
            appFiles.map(async ({ filename, type }) => {
              const sha = shaMap[filename];
              if (!sha) {
                console.warn(
                  `Warning: Package ${filename} not found in ${app.repo} SHA256SUMS.txt`
                );
                return;
              }

              const existingEntry = existingData[filename];
              if (
                !isForce &&
                existingEntry &&
                existingEntry.sha256 === sha &&
                existingEntry.size > 0
              ) {
                nextData[filename] = existingEntry;
                return;
              }

              // Fetch size via HTTP HEAD
              let size = 0;
              try {
                const headUrl = `https://github.com/storytold/${app.repo}/releases/download/v${app.version}/${filename}`;
                const headRes = await fetch(headUrl, {
                  method: "HEAD",
                  redirect: "follow",
                });
                if (headRes.ok) {
                  const contentLength = headRes.headers.get("content-length");
                  if (contentLength) {
                    size = Number(contentLength);
                  }
                }
              } catch (headErr) {
                console.warn(`Warning: Could not fetch HEAD for ${filename}:`, headErr);
              }

              if (size === 0 && existingEntry?.size) {
                size = existingEntry.size;
              }

              let installedSize = 0;
              if (existingEntry?.installedSize) {
                installedSize = existingEntry.installedSize;
              } else if (type === "deb") {
                installedSize = Math.round((size * 1.95) / 1024);
              } else if (type === "rpm") {
                installedSize = Math.round(size * 1.95);
              } else if (type === "arch") {
                try {
                  const headUrl = `https://github.com/storytold/${app.repo}/releases/download/v${app.version}/${filename}`;
                  const cmd = `curl -sL -r 0-65535 "${headUrl}" | zstd -dc 2>/dev/null | tar -x -O .PKGINFO 2>/dev/null | grep "^size ="`;
                  const out = execSync(cmd, { encoding: "utf-8" }).trim();
                  const match = out.match(/^size\s*=\s*(\d+)/m);
                  if (match) installedSize = Number(match[1]);
                } catch {
                  // Fall back to size estimate
                }
                if (!installedSize) installedSize = Math.round(size * 2.5);
              }

              nextData[filename] = {
                size,
                sha256: sha,
                installedSize,
              };
            })
          );
        } catch (fetchErr) {
          console.warn(
            `Warning: Failed to synchronize metadata for ${app.name} ${app.version}:`,
            fetchErr
          );
        }
      })
    );
  }

  // Sort keys alphabetically for clean diffs
  const sortedData = {};
  for (const key of Object.keys(nextData).sort()) {
    sortedData[key] = nextData[key];
  }

  writeFileSync(
    packageDataJsonPath,
    JSON.stringify(sortedData, null, 2) + "\n",
    "utf-8"
  );

  const duration = Date.now() - startTime;
  console.log(
    `✓ Synchronized ${Object.keys(sortedData).length} package entries in ${packageDataJsonPath} (${duration}ms)`
  );
}

main().catch((err) => {
  console.error("Fatal error during package sync:", err);
  process.exit(1);
});
