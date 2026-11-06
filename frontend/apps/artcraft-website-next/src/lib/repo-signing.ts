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

/** Base64 of an armored key, as a single-line secret is often stored. Null when it is not one. */
function decodeBase64Key(value: string): string | null {
  const compact = value.replaceAll(/\s/g, "");
  // A base64 length of 4n+1 cannot be valid, and a value without padding is still one copy away
  // from usable, so pad it rather than rejecting it.
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(compact) || compact.length % 4 === 1) {
    return null;
  }
  const padded = compact.padEnd(Math.ceil(compact.length / 4) * 4, "=");
  const decoded = Buffer.from(padded, "base64").toString("utf-8");
  return decoded.includes("-----BEGIN PGP") ? decoded : null;
}

/**
 * Escaped newlines, single (JSON, .env, `netlify env:set`) or doubled (a JSON string nested in a
 * JSON value). Only a single-line value is rewritten: an armored block that already has real
 * newlines is valid as-is, and rewriting a backslash inside it (a `Comment:` header may contain
 * one, and a literal `\n` in it is not an escaped newline) would break the armor.
 */
function unescapeSingleLineKey(value: string): string {
  if (value.includes("\n")) {
    return value;
  }
  return value.replaceAll("\\\\", "\\").replaceAll("\\r\\n", "\n").replaceAll("\\n", "\n");
}

/**
 * Accept the shapes a multi-line secret arrives in: an armored block with real newlines, the same
 * block on one line with `\n` escapes (JSON, `.env`, `netlify env:set`), its base64 encoding, and
 * either of those wrapped in matching quotes. openpgp.js only parses the first, and a value it
 * cannot read silently disables signing, so normalize before parsing.
 */
export function normalizeArmoredKey(raw: string): string {
  let value = raw.trim();
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    value = value.slice(1, -1).trim();
  }
  // Decode base64 first, so the armor that comes out goes through the same handling as a value
  // stored in the clear: base64 of a single-line escaped key still needs its newlines restored.
  value = decodeBase64Key(value) ?? value;
  return unescapeSingleLineKey(value).replaceAll("\r\n", "\n");
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
      // involved (rpm, debsigs, flatpak, skopeo) would otherwise need a pinentry. A protected key
      // parses, so reject it here rather than letting every signature request fail.
      const privateKey = await openpgp.readPrivateKey({ armoredKey: normalizeArmoredKey(armored) });
      if (!privateKey.isDecrypted()) {
        throw new Error("LINUX_SIGNING_KEY is passphrase-protected; export it without a passphrase");
      }
      // A key that cannot sign must not make the repository configs advertise signatures either,
      // and a protected signing subkey fails at sign time even when the primary key is clear.
      const signingKey = await privateKey.getSigningKey();
      if (!signingKey.isDecrypted()) {
        throw new Error(
          "LINUX_SIGNING_KEY has a passphrase-protected signing subkey; export it without a passphrase",
        );
      }
      const publicKey = privateKey.toPublic();
      const fingerprint = publicKey.getFingerprint().toUpperCase();
      console.info(`Linux repository signing enabled with key ${fingerprint}`);
      return { privateKey, publicKey, fingerprint };
    } catch (error) {
      console.error(
        "Unable to read LINUX_SIGNING_KEY; the Linux repositories will be published unsigned",
        error,
      );
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

/**
 * Binary signed message: the payload travels inside the OpenPGP message with its signature. This is
 * the shape containers/image uses for OCI signatures, which Flatpak reads from a signature
 * lookaside, so the client gets the signed payload and the signature in one file.
 */
export async function signInlineBinary(payload: Uint8Array): Promise<Uint8Array | null> {
  const key = await loadSigningKey();
  if (!key) {
    return null;
  }
  try {
    const message = await openpgp.createMessage({ binary: payload });
    // Binary literal data: no filename and no text canonicalization, so the client verifies the
    // exact bytes of the payload.
    return (await openpgp.sign({
      message,
      signingKeys: key.privateKey,
      detached: false,
      format: "binary",
    })) as Uint8Array;
  } catch (error) {
    console.error("Unable to sign an OCI image signature", error);
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
