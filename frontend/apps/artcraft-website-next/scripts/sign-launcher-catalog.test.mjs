import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { generateKeyPairSync, verify } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { signLauncherCatalog } from "./sign-launcher-catalog.mjs";

function fixture(t, type = "ed25519") {
  const directory = mkdtempSync(join(tmpdir(), "launcher-signing-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const { privateKey, publicKey } = generateKeyPairSync(type, type === "rsa" ? { modulusLength: 2048 } : {});
  const environment = {
    LAUNCHER_CATALOG_SIGNING_KEY: Buffer.from(privateKey.export({ type: "pkcs8", format: "pem" })).toString("base64"),
  };
  const catalog = Buffer.from('[[apps]]\nid = "photocraft"\n');
  writeFileSync(join(directory, "launcher-catalog.toml"), catalog);
  return { directory, environment, publicKey, catalog };
}

test("signature verifies exact bytes with the public key and rejects tampering", (t) => {
  const { directory, environment, publicKey, catalog } = fixture(t);
  signLauncherCatalog(directory, environment);
  const signature = Buffer.from(readFileSync(join(directory, "launcher-catalog.toml.sig"), "utf8").trim(), "base64");
  assert.equal(signature.length, 64);
  assert.equal(verify(null, catalog, publicKey, signature), true);
  assert.equal(verify(null, Buffer.concat([catalog, Buffer.from("\n")]), publicKey, signature), false);
  assert.equal(verify(null, catalog, generateKeyPairSync("ed25519").publicKey, signature), false);
});

test("missing or malformed keys fail and remove stale signatures", (t) => {
  const { directory } = fixture(t);
  for (const environment of [{}, { LAUNCHER_CATALOG_SIGNING_KEY: "invalid" }]) {
    writeFileSync(join(directory, "launcher-catalog.toml.sig"), "stale");
    assert.throws(() => signLauncherCatalog(directory, environment), /LAUNCHER_CATALOG_SIGNING_KEY/);
    assert.equal(existsSync(join(directory, "launcher-catalog.toml.sig")), false);
  }
});

test("other key algorithms are rejected", (t) => {
  const { directory, environment } = fixture(t, "rsa");
  assert.throws(() => signLauncherCatalog(directory, environment), /must be an Ed25519 key/);
});

test("CLI rejects arbitrary output directories before touching files", (t) => {
  const { directory } = fixture(t);
  const signaturePath = join(directory, "launcher-catalog.toml.sig");
  writeFileSync(signaturePath, "keep");
  const result = spawnSync(process.execPath, [fileURLToPath(new URL("./sign-launcher-catalog.mjs", import.meta.url)), directory], { encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /no path arguments are accepted/);
  assert.equal(readFileSync(signaturePath, "utf8"), "keep");
});
