import { createPrivateKey, createPublicKey, sign, verify } from "node:crypto";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Detached Ed25519 signature over the exact deployed TOML bytes, encoded as base64.
// The private key is a build secret; only the signature is written to the output.
export function signLauncherCatalog(directory, environment = process.env) {
  const signaturePath = resolve(directory, "launcher-catalog.toml.sig");
  // Never leave a previous build's signature beside a changed catalog.
  rmSync(signaturePath, { force: true });
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
  const catalog = readFileSync(resolve(directory, "launcher-catalog.toml"));
  const signature = sign(null, catalog, key);
  if (!verify(null, catalog, createPublicKey(key), signature)) {
    throw new Error("Launcher catalog signature verification failed");
  }
  writeFileSync(signaturePath, signature.toString("base64") + "\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  signLauncherCatalog(resolve(process.argv[2] ?? "out"));
  console.info("Signed launcher-catalog.toml (Ed25519)");
}
