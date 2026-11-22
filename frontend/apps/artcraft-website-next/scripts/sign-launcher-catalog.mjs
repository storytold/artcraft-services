import { createPrivateKey, createPublicKey, sign, verify } from "node:crypto";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

// The launcher's catalog, and its announcements and alerts: it shows notices only from a
// signed file (craft-launcher docs/adr/0002).
export const SIGNED_FILES = ["launcher-catalog.toml", "launcher-announcements.toml"];

// Detached Ed25519 signature over each file's exact deployed bytes, encoded as base64.
// The private key is a build secret; only the signatures are written to the output.
export function signLauncherCatalog(directory, environment = process.env) {
  // Never leave a previous build's signature beside a changed file.
  for (const name of SIGNED_FILES) {
    rmSync(resolve(directory, `${name}.sig`), { force: true });
  }
  const encoded = environment.LAUNCHER_CATALOG_SIGNING_KEY;
  if (!encoded) {
    throw new Error("LAUNCHER_CATALOG_SIGNING_KEY must contain a base64-encoded Ed25519 PKCS#8 PEM private key");
  }
  let key;
  try {
    key = createPrivateKey(Buffer.from(encoded, "base64").toString("utf8"));
  } catch {
    throw new Error("LAUNCHER_CATALOG_SIGNING_KEY is not a valid base64-encoded PKCS#8 PEM private key");
  }
  if (key.asymmetricKeyType !== "ed25519") {
    throw new Error("LAUNCHER_CATALOG_SIGNING_KEY must be an Ed25519 key");
  }
  for (const name of SIGNED_FILES) {
    const bytes = readFileSync(resolve(directory, name));
    const signature = sign(null, bytes, key);
    if (!verify(null, bytes, createPublicKey(key), signature)) {
      throw new Error(`${name} signature verification failed`);
    }
    writeFileSync(resolve(directory, `${name}.sig`), signature.toString("base64") + "\n");
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length > 2) {
    throw new Error("This command signs only the fixed out/ deployment directory; no path arguments are accepted");
  }
  signLauncherCatalog(resolve("out"));
  console.info(`Signed ${SIGNED_FILES.join(" and ")} (Ed25519)`);
}
