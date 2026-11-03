import { createMessage, readKey, readSignature, verify } from "openpgp";

/**
 * The ArtCraft Linux release key's ASCII-armored public key (`release-signing-key.asc` in every
 * signed release). craft-common-actions signs each release's Linux packages, Flatpak bundle and
 * OCI images, and SHA256SUMS.txt with its signing subkey.
 * Primary key 2B76 741B 414A AD6C D33E  6312 13A0 05C0 B9D7 C3ED,
 * signing subkey 568D C1F6 8F8F AECC 797F  3C7B 006D AF6B B0EF 334E.
 */
export const RELEASE_SIGNING_KEY = `-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEaulK1xYJKwYBBAHaRw8BAQdAqHJeDCjY6Hx5FTn/hW+aqu6V394ubLk0Zj3C
bftDwFm0IkFydENyYWZ0IExpbnV4IFJlbGVhc2UgU2lnbmluZyBLZXmIkwQTFgoA
OxYhBCt2dBtBSq1s0z5jEhOgBcC518PtBQJq6UrXAhsBBQsJCAcCAiICBhUKCQgL
AgQWAgMBAh4HAheAAAoJEBOgBcC518PtGeUA/RjvULBTLlccc+2wtJkfjSnCV+cf
PUMEGmW7XkZWkk51AP9w2A/WLv+79nREWEpZzI8LGK5EjrNazP+sY0l7j5rFCLgz
BGrpStcWCSsGAQQB2kcPAQEHQPl7XkQ1gWsvSPf/T9b4m/UHcjJjBpSOtpYlWR56
er5RiO8EGBYKACAWIQQrdnQbQUqtbNM+YxIToAXAudfD7QUCaulK1wIbAgCBCRAT
oAXAudfD7XYgBBkWCgAdFiEEVo3B9o+Prsx5fzx7AG2va7DvM04FAmrpStcACgkQ
AG2va7DvM07taQEA4d2VFkKRIP0g9TrE86KpnuXPap+LKqe0tfIxpdf9YxIA/0Wo
9H/iG8nH++nxC087p3HE0S+0UovxqkhXJLSr8B4JDOoA/iLRpqTdYmk6fVXMyPJo
SkOVuwZIGS6VgBdSQP0e2P2YAP41L5OoSvwdQIG3/uNrM+ieqpYeM597/hCLFs5A
ga/qCA==
=3wGQ
-----END PGP PUBLIC KEY BLOCK-----
`;

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
