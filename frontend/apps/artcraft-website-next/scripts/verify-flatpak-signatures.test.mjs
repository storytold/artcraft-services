import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createServer } from "node:http";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { test } from "node:test";
import * as openpgp from "openpgp";

const run = promisify(execFile);
const verifier = fileURLToPath(new URL("./verify-flatpak-signatures.mjs", import.meta.url));

test("verifies deployed digests and rejects missing, corrupt, or mismatched signatures", async () => {
  const { privateKey, publicKey } = await openpgp.generateKey({
    type: "ecc",
    curve: "curve25519Legacy",
    userIDs: [{ name: "Repository fixture" }],
    format: "object",
  });
  const digest = `sha256:${"a".repeat(64)}`;
  const repository = "fixture/app";
  let mode = "valid";
  let baseUrl;
  const payload = (imageDigest) => JSON.stringify({
    critical: {
      type: "atomic container signature",
      image: { "docker-manifest-digest": imageDigest },
      identity: { "docker-reference": `ghcr.io/${repository}` },
    },
  });
  const sign = async (data) => openpgp.sign({
    message: await openpgp.createMessage({ text: data }),
    signingKeys: privateKey,
    format: "binary",
  });
  const valid = await sign(payload(digest));
  const mismatch = await sign(payload(`sha256:${"b".repeat(64)}`));
  const corrupt = new Uint8Array(valid);
  corrupt[corrupt.length - 1] ^= 1;
  const server = createServer((request, response) => {
    if (request.url === "/artcraft-oci.flatpakrepo") {
      response.end(`[Flatpak Repo]\nGPGKey=${Buffer.from(publicKey.write()).toString("base64")}\nSignatureLookaside=${baseUrl}/oci-signatures\n`);
    } else if (request.url === "/index/static") {
      response.end(JSON.stringify({ Registry: "https://ghcr.io", Results: [{
        Repository: repository, Images: [{ Digest: digest, Tags: ["1.0-x86_64"] }],
      }] }));
    } else if (request.url === `/oci-signatures/${repository}@${digest.replace(":", "=")}/signature-1` && mode !== "missing") {
      response.end(Buffer.from(mode === "mismatch" ? mismatch : mode === "corrupt" ? corrupt : valid));
    } else {
      response.writeHead(404).end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  baseUrl = `http://127.0.0.1:${server.address().port}`;
  try {
    const result = await run(process.execPath, [verifier, baseUrl]);
    assert.match(result.stdout, /Verified 1 OCI image signatures/);
    for (mode of ["missing", "corrupt", "mismatch"]) {
      await assert.rejects(run(process.execPath, [verifier, baseUrl]), `${mode} must fail`);
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
