import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { copyLauncherAssets } from "./fetch-launcher-assets.mjs";

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), "launcher-assets-test-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const repo = join(dir, "repo"), outDir = join(dir, "out");
  mkdirSync(repo);
  const git = (...args) => execFileSync("git", ["-C", repo, ...args], { stdio: "pipe" }).toString().trim();
  git("init", "-b", "main");
  git("config", "user.name", "Test");
  git("config", "user.email", "test@example.invalid");
  mkdirSync(join(repo, "photocraft"));
  writeFileSync(join(repo, "photocraft/demo.jpg"), "original screenshot");
  writeFileSync(join(repo, "ATTRIBUTION.md"), "original attribution");
  git("add", "."); git("commit", "-m", "Screenshot v1"); git("tag", "v1");
  writeFileSync(join(repo, "photocraft/demo.jpg"), "updated screenshot");
  git("add", "."); git("commit", "-m", "Screenshot v2"); git("tag", "v2");
  mkdirSync(join(repo, "app-icons"));
  const icon = Buffer.from("icon bytes");
  const hash = createHash("sha256").update(icon).digest("hex");
  const filename = "photocraft-" + hash + ".png";
  writeFileSync(join(repo, "app-icons", filename), icon);
  writeFileSync(join(repo, "app-icons/ATTRIBUTION.md"), "icon attribution");
  writeFileSync(join(repo, "catalog.toml"), '[[apps]]\nid = "photocraft"\nicon_url = "https://getartcraft.com/launcher-app-icons/' + filename + '"\nicon_sha256 = "' + hash + '"\n[[apps.details.shots]]\nurl = "https://getartcraft.com/launcher-screenshots/v1/photocraft/demo.jpg"\n');
  writeFileSync(join(repo, "ignored.js"), "not a deployable asset");
  git("add", "."); git("commit", "-m", "Catalog");
  return { repo, outDir, git, filename };
}

test("copies one committed catalog revision and retains immutable screenshot versions", (t) => {
  const { repo, outDir, git, filename } = fixture(t);
  const commit = git("rev-parse", "HEAD");
  writeFileSync(join(repo, "catalog.toml"), "uncommitted edit must not be read");
  mkdirSync(join(outDir, "launcher-app-icons"), { recursive: true });
  writeFileSync(join(outDir, "launcher-app-icons", "stale.png"), "old generated output");
  const result = copyLauncherAssets({ repo, outDir });
  assert.equal(existsSync(join(outDir, "launcher-app-icons", "stale.png")), false);
  assert.equal(result.commit, commit);
  assert.match(readFileSync(join(outDir, "launcher-catalog.toml"), "utf8"), new RegExp(commit));
  assert.equal(
    readFileSync(join(outDir, "..", ".launcher-asset-cache", "launcher-catalog.base.toml"), "utf8"),
    readFileSync(join(outDir, "launcher-catalog.toml"), "utf8"),
  );
  assert.equal(readFileSync(join(outDir, "launcher-app-icons", filename), "utf8"), "icon bytes");
  assert.equal(readFileSync(join(outDir, "launcher-screenshots/v1/photocraft/demo.jpg"), "utf8"), "original screenshot");
  assert.equal(readFileSync(join(outDir, "launcher-screenshots/v2/photocraft/demo.jpg"), "utf8"), "updated screenshot");
  assert.equal(readFileSync(join(outDir, "launcher-app-icons/ATTRIBUTION.md"), "utf8"), "icon attribution");
  assert.equal(existsSync(join(outDir, "ignored.js")), false);
});

test("rejects a catalog whose referenced icon no longer matches its digest", (t) => {
  const { repo, outDir, git, filename } = fixture(t);
  writeFileSync(join(repo, "app-icons", filename), "changed icon");
  git("add", "."); git("commit", "-m", "Wrong hash");
  assert.throws(() => copyLauncherAssets({ repo, outDir }), /Icon digest does not match/);
});

test("rejects a catalog that references a screenshot the published tags do not contain", (t) => {
  const { repo, outDir, git } = fixture(t);
  writeFileSync(join(repo, "catalog.toml"), readFileSync(join(repo, "catalog.toml"), "utf8").replace(
    "v1/photocraft/demo.jpg",
    "v1/photocraft/missing.jpg",
  ));
  git("add", "."); git("commit", "-m", "Missing screenshot");
  assert.throws(() => copyLauncherAssets({ repo, outDir }), /screenshot references do not match/);
});

