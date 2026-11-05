import * as openpgp from "openpgp";

/**
 * Signing for the Linux package repositories.
 *
 * The private key is the same one the release pipelines use (LINUX_SIGNING_KEY), supplied as an
 * environment variable so the repositories can sign their metadata at request time: apt needs a
 * signed Release (or InRelease), dnf verifies repomd.xml against repomd.xml.asc, and pacman checks
 * artcraft.db against artcraft.db.sig. Without the key the routes publish unsigned metadata, which
 * is what previews and local builds get.
 */

type SigningKey = {
  privateKey: openpgp.PrivateKey;
  publicKey: openpgp.Key;
  fingerprint: string;
};

function signingKeyArmored(): string {
  return process.env.LINUX_SIGNING_KEY?.trim() ?? "";
}

/**
 * Whether this deployment can sign repository metadata. Checks that the key actually loads, so a
 * malformed LINUX_SIGNING_KEY does not leave clients requiring signatures that are never served.
 */
export async function isRepoSigningConfigured(): Promise<boolean> {
  return (await loadSigningKey()) !== null;
}

let cachedKey: { armored: string; key: Promise<SigningKey | null> } | null = null;

/** Parses the key once per process; a useless key is reported and treated as "not configured". */
async function loadSigningKey(): Promise<SigningKey | null> {
  const armored = signingKeyArmored();
  if (!armored) {
    return null;
  }
  if (cachedKey?.armored === armored) {
    return cachedKey.key;
  }

  const key = (async () => {
    try {
      // The key must be unprotected: the release pipelines sign with the same one and every tool
      // involved (rpm, debsigs, flatpak, skopeo) would otherwise need a pinentry.
      const privateKey = await openpgp.readPrivateKey({ armoredKey: armored });
      const publicKey = privateKey.toPublic();
      return { privateKey, publicKey, fingerprint: publicKey.getFingerprint().toUpperCase() };
    } catch (error) {
      console.error("Unable to read LINUX_SIGNING_KEY; signing the Linux repositories is disabled", error);
      return null;
    }
  })();
  cachedKey = { armored, key };
  return key;
}

/** Armored detached signature, as apt's Release.gpg and dnf's repomd.xml.asc expect. */
export async function signDetachedArmored(content: string): Promise<string | null> {
  const key = await loadSigningKey();
  if (!key) {
    return null;
  }
  try {
    // Binary mode, not text: gpgv canonicalizes line endings for text signatures, but rpm's own
    // verifier (which dnf uses for repomd.xml) hashes the served bytes exactly as they arrive.
    const message = await openpgp.createMessage({ binary: Buffer.from(content, "utf-8") });
    return (await openpgp.sign({
      message,
      signingKeys: key.privateKey,
      detached: true,
      format: "armored",
    })) as string;
  } catch (error) {
    console.error("Unable to sign repository metadata", error);
    return null;
  }
}

/** Binary detached signature, as pacman's artcraft.db.sig expects. */
export async function signDetachedBinary(content: Uint8Array): Promise<Uint8Array | null> {
  const key = await loadSigningKey();
  if (!key) {
    return null;
  }
  try {
    const message = await openpgp.createMessage({ binary: content });
    // pacman wants the raw signature packet; without format: "binary" openpgp.js armors it.
    const signed = await openpgp.sign({
      message,
      signingKeys: key.privateKey,
      detached: true,
      format: "binary",
    });
    return signed as Uint8Array;
  } catch (error) {
    console.error("Unable to sign repository metadata", error);
    return null;
  }
}

/** Clear-signed release file: apt prefers InRelease and falls back to Release + Release.gpg. */
export async function signCleartext(content: string): Promise<string | null> {
  const key = await loadSigningKey();
  if (!key) {
    return null;
  }
  try {
    const message = await openpgp.createCleartextMessage({ text: content });
    return (await openpgp.sign({ message, signingKeys: key.privateKey })) as string;
  } catch (error) {
    console.error("Unable to sign the apt Release file", error);
    return null;
  }
}

/** The public half, for clients to import before trusting the repositories. */
export async function getSigningPublicKey(): Promise<{
  armored: string;
  binary: Uint8Array;
  fingerprint: string;
} | null> {
  const key = await loadSigningKey();
  if (!key) {
    return null;
  }
  return {
    armored: key.publicKey.armor(),
    binary: key.publicKey.write(),
    fingerprint: key.fingerprint,
  };
}
