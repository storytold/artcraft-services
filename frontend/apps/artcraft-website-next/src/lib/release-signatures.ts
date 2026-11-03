import { createMessage, readKey, readSignature, verify } from "openpgp";

/**
 * The ArtCraft release key's ASCII-armored public key (`release-signing-key.asc` in every signed
 * release). craft-common-actions signs each release's Linux packages and SHA256SUMS.txt with it.
 * Empty: checksums are trusted without a signature check.
 */
export const RELEASE_SIGNING_KEY = "";

/** The binary public key, base64-encoded, as `.flatpakrepo` and `.flatpakref` files take it (`GPGKey=`). */
export async function flatpakGpgKey(armoredKey: string): Promise<string> {
  const key = await readKey({ armoredKey });
  return Buffer.from(key.write()).toString("base64");
}

/** Does `armoredSignature` hold a detached signature over `data` by `armoredKey`? */
export async function verifyDetachedSignature(
  data: Uint8Array,
  armoredSignature: string,
  armoredKey: string,
): Promise<boolean> {
  try {
    const [verificationKeys, signature, message] = await Promise.all([
      readKey({ armoredKey }),
      readSignature({ armoredSignature }),
      createMessage({ binary: data }),
    ]);
    const { signatures } = await verify({ message, signature, verificationKeys });
    // A signature by any other key, or over other bytes, rejects.
    const results = await Promise.allSettled(signatures.map((s) => s.verified));
    return results.some((r) => r.status === "fulfilled" && r.value === true);
  } catch {
    return false;
  }
}

/**
 * Parses a release's SHA256SUMS.txt (filename -> sha256) after checking `armoredSignature`
 * (SHA256SUMS.txt.asc) with `armoredKey`. Throws when a signature is there but doesn't verify.
 */
export async function parseSignedShaSums(
  data: Uint8Array,
  armoredSignature: string | null,
  armoredKey: string = RELEASE_SIGNING_KEY,
): Promise<Map<string, string>> {
  // shortcut: a release with no SHA256SUMS.txt.asc (published before signing) is still trusted; require it once every catalogued release is signed.
  if (armoredKey && armoredSignature !== null && !(await verifyDetachedSignature(data, armoredSignature, armoredKey))) {
    throw new Error("SHA256SUMS.txt.asc does not verify with the ArtCraft release key");
  }
  const map = new Map<string, string>();
  for (const line of new TextDecoder().decode(data).split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const [sha, fname] = trimmed.split(/\s+/);
    if (sha && fname) map.set(fname.replace(/^\*/, ""), sha);
  }
  return map;
}
