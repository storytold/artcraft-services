import * as openpgp from "openpgp";

const baseUrl = process.argv[2]?.replace(/\/$/, "");
if (!baseUrl) throw new Error("Usage: node scripts/verify-flatpak-signatures.mjs BASE_URL");

/** Fetches a deployed artifact, rejecting missing signatures and other HTTP failures. */
async function fetchArtifact(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return response;
}

const descriptor = await (await fetchArtifact(`${baseUrl}/artcraft-oci.flatpakrepo`)).text();
const keyBase64 = /^GPGKey=(.+)$/m.exec(descriptor)?.[1].trim();
const lookaside = /^SignatureLookaside=(.+)$/m.exec(descriptor)?.[1].trim();
if (!keyBase64) throw new Error("The Flatpak remote descriptor publishes no GPG key");
if (lookaside !== `${baseUrl}/oci-signatures`) {
  throw new Error(`Unexpected Flatpak signature lookaside: ${lookaside}`);
}
const verificationKeys = await openpgp.readKeys({ binaryKeys: Buffer.from(keyBase64, "base64") });
const index = await (await fetchArtifact(`${baseUrl}/index/static`)).json();
const registryHost = new URL(index.Registry).host;
if (!index.Results?.length) throw new Error("The deployed Flatpak index has no repositories");

// Verify exactly the immutable digests advertised by this deployment. A mutable stable tag can
// advance independently of the deployed index and is not the image Flatpak is instructed to pull.
let verified = 0;
for (const repo of index.Results) {
  if (!repo.Images?.length) throw new Error(`No indexed images for ${repo.Repository}`);
  for (const digest of new Set(repo.Images.map((image) => image.Digest))) {
    if (!/^sha256:[a-f0-9]{64}$/.test(digest)) throw new Error(`Invalid OCI digest: ${digest}`);
    const url = `${lookaside}/${repo.Repository}@${digest.replace(":", "=")}/signature-1`;
    const bytes = await (await fetchArtifact(url)).arrayBuffer();
    const message = await openpgp.readMessage({ binaryMessage: new Uint8Array(bytes) });
    const result = await openpgp.verify({ message, verificationKeys });
    if (!result.signatures.length) throw new Error(`No OpenPGP signature for ${digest}`);
    await Promise.all(result.signatures.map((signature) => signature.verified));
    const payload = JSON.parse(result.data);
    if (
      payload.critical?.type !== "atomic container signature" ||
      payload.critical?.image?.["docker-manifest-digest"] !== digest ||
      payload.critical?.identity?.["docker-reference"] !== `${registryHost}/${repo.Repository}`
    ) {
      throw new Error(`Signature does not cover ${repo.Repository}@${digest}`);
    }
    verified++;
  }
}
console.info(`Verified ${verified} OCI image signatures against the deployed descriptor key and index.`);
