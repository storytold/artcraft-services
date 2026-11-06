import {
  getFlatpakOciImageDigests,
  getFlatpakOciRegistryOrigin,
} from "./flatpak-oci";
import {
  getSigningPublicKey,
  signInlineBinary,
} from "./repo-signing";

/**
 * OCI image signatures for the Flatpak remote.
 *
 * Flatpak verifies OCI pulls with signatures fetched from a containers/image signature lookaside:
 * `<lookaside>/<repository>@sha256=<hex>/signature-1`, each holding a PGP-signed message whose
 * payload is the atomic signature JSON. The payload names the exact manifest digest and the
 * repository the image came from, and Flatpak compares both against what it is installing, so the
 * signature is built from the index's resolved digest rather than from the request.
 */

/** Path the Flatpak descriptors point at for signatures. */
export const OCI_SIGNATURE_LOOKASIDE_PATH = "/oci-signatures";

const SIGNATURE_CACHE_MS = 60 * 60 * 1000;
const signatureCache = new Map<string, { signature: Promise<Uint8Array | null>; expiresAt: number }>();

export type OciSignatureLookasideRequest = {
  repository: string;
  /** The digest in `sha256:<hex>` form. */
  digest: string;
  /** `signature-<n>` index; one signature per image, so only 1 is published. */
  index: number;
};

/**
 * containers/image atomic signature payload, with the identity Flatpak expects: the registry host
 * from the index plus the OCI repository name (`ghcr.io/storytold/pdfcraft`).
 */
export function buildOciSignaturePayload(
  digest: string,
  repository: string,
  createdAt: Date = new Date(),
): string {
  const registryHost = new URL(getFlatpakOciRegistryOrigin()).host;
  return JSON.stringify({
    critical: {
      type: "atomic container signature",
      image: { "docker-manifest-digest": digest },
      identity: { "docker-reference": `${registryHost}/${repository}` },
    },
    optional: {
      creator: "artcraft-website",
      timestamp: Math.floor(createdAt.getTime() / 1000),
    },
  });
}

/**
 * Parse `<repository>@sha256=<hex>/signature-<n>` as Flatpak builds it from the index's repository
 * name. Anything else is not a lookaside request this endpoint answers.
 */
export function parseSignatureLookasidePath(segments: string[]): OciSignatureLookasideRequest | null {
  if (segments.length < 2) {
    return null;
  }
  const indexMatch = /^signature-(\d+)$/.exec(segments.at(-1)!);
  if (!indexMatch) {
    return null;
  }

  const reference = segments.slice(0, -1).join("/");
  const digestSeparator = reference.lastIndexOf("@");
  if (digestSeparator === -1) {
    return null;
  }

  const repository = reference.slice(0, digestSeparator);
  const digestMatch = /^sha256=([a-f0-9]{64})$/.exec(reference.slice(digestSeparator + 1));
  if (!digestMatch) {
    return null;
  }
  // A repository name is `namespace/name` with at least one slash and no traversal.
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*(?:\/[A-Za-z0-9][A-Za-z0-9._-]*)+$/.test(repository)) {
    return null;
  }

  return {
    repository,
    digest: `sha256:${digestMatch[1]}`,
    index: Number(indexMatch[1]),
  };
}

/**
 * A signed message for a published image, or null when the repository is not catalogued or the
 * digest is not one the index currently advertises.
 */
export async function getOciImageSignature(
  repository: string,
  digest: string,
): Promise<Uint8Array | null> {
  const key = `${repository}@${digest}`;
  const cached = signatureCache.get(key);
  if (cached && cached.expiresAt > Date.now()) {
    return cached.signature;
  }

  const signature = (async () => {
    let published: string[] | null;
    try {
      published = await getFlatpakOciImageDigests(repository);
    } catch (error) {
      // A registry that cannot be reached now publishes nothing to sign; Flatpak retries the pull.
      console.error(`Unable to resolve OCI digests for ${repository}`, error);
      return null;
    }
    if (!published?.includes(digest)) {
      return null;
    }
    return signInlineBinary(Buffer.from(buildOciSignaturePayload(digest, repository), "utf-8"));
  })();

  signatureCache.set(key, { signature, expiresAt: Date.now() + SIGNATURE_CACHE_MS });
  signature.catch(() => signatureCache.delete(key));
  const value = await signature;
  if (!value) {
    // A digest the index does not publish yet may appear after the next release resolution.
    signatureCache.delete(key);
  }
  return value;
}

/**
 * `GPGKey` and `SignatureLookaside` descriptor entries. Flatpak turns verification on as soon as a
 * `.flatpakrepo` or `.flatpakref` carries a key, and reads the signatures from the lookaside.
 */
export async function getFlatpakSignatureDescriptor(baseUrl: string): Promise<string[]> {
  const key = await getSigningPublicKey();
  if (!key) {
    return [];
  }
  return [
    `GPGKey=${Buffer.from(key.binary).toString("base64")}`,
    `SignatureLookaside=${baseUrl}${OCI_SIGNATURE_LOOKASIDE_PATH}`,
  ];
}
