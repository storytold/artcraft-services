import { expect, test } from "@playwright/test";
import {
  generateAptPackages,
  generateAptRelease,
  generateAptSourcesList,
  generateAptSourcesDeb822,
  generateRpmPrimaryXml,
  generateRpmRepomdXml,
  generateRpmRepoConfig,
  generatePacmanDb,
  generatePacmanDesc,
  generatePacmanRepoConfig,
  getRpmPrimaryXml,
  getRpmPrimaryGz,
  getRpmRepomdXml,
  getLinuxRepoApps,
  resolvePackageDownloadUrl,
  withholdUnsignedPackages,
  RPM_REPO_TIMESTAMP,
  VALID_APT_DISTRIBUTIONS,
  VALID_APT_COMPONENTS,
  VALID_APT_ARCHES,
  VALID_RPM_ARCHES,
  VALID_PACMAN_ARCHES,
  escapeXml,
  getRequestBaseUrl,
  buildTar,
  createPkgInfoScanner,
} from "../src/lib/linux-repos";
import * as openpgp from "openpgp";
import { createHash } from "node:crypto";
import { getSigningPublicKey, isRepoSigningConfigured, normalizeArmoredKey, signCleartext, signDetachedArmored, signDetachedBinary } from "../src/lib/repo-signing";
import { CRAFTING_APPS, craftAppRelease, craftReleaseDownloads } from "../src/lib/crafting-apps";
import { CRAFT_LAUNCHER_RELEASE } from "../src/lib/crafting-app-releases";
import { GET as getInRelease } from "../src/app/apt/dists/[distribution]/InRelease/route";
import { GET as getReleaseGpg } from "../src/app/apt/dists/[distribution]/Release.gpg/route";
import { GET as getRpmSignature } from "../src/app/rpm/[arch]/repodata/repomd.xml.asc/route";
import { GET as getArchSignature } from "../src/app/arch/[arch]/artcraft.db.sig/route";
import { GET as getArchPackage } from "../src/app/arch/[arch]/[filename]/route";
import { GET as getKeyAsc } from "../src/app/keys/artcraft.asc/route";
import { GET as getKeyGpg } from "../src/app/keys/artcraft.gpg/route";

// Read versions and assets from the release catalog so release bumps don't break these tests.
function catalogRelease(slug: string) {
  const app = CRAFTING_APPS.find((a) => a.slug === slug);
  const release = app && craftAppRelease(app);
  if (!app || !release) throw new Error(`No catalogued release for ${slug}`);
  const downloads = craftReleaseDownloads(app, release);
  const download = (fileName: string) => {
    const match = downloads.find((d) => d.fileName === fileName);
    if (!match) throw new Error(`${fileName} is not in the ${slug} ${release.version} release`);
    return match.href;
  };
  const count = (suffix: string) => downloads.filter((d) => d.fileName.endsWith(suffix)).length;
  return { version: release.version, download, count };
}

const pdfcraft = catalogRelease("pdfcraft");
const craftAnything = catalogRelease("craft-anything");
const launcherVersion = CRAFT_LAUNCHER_RELEASE!.version;
const pdf = pdfcraft.version;

test.describe("Linux Package Repositories (APT, RPM & Pacman)", () => {
  test.beforeEach(() => {
    // These tests assert the unsigned output, even if the process inherited a signing key.
    delete process.env.LINUX_SIGNING_KEY;
  });

  test("resolves release download URLs only for catalogued package assets", () => {
    const pdfBase = `https://github.com/storytold/pdfcraft/releases/download/v${pdf}`;
    const launcherBase = `https://github.com/storytold/craft-launcher/releases/download/v${launcherVersion}`;

    const debUrl = resolvePackageDownloadUrl(`pdfcraft-${pdf}-linux-x86_64.deb`);
    expect(debUrl).toBe(`${pdfBase}/pdfcraft-${pdf}-linux-x86_64.deb`);

    const rpmUrl = resolvePackageDownloadUrl(`pdfcraft-${pdf}-linux-x86_64.rpm`);
    expect(rpmUrl).toBe(`${pdfBase}/pdfcraft-${pdf}-linux-x86_64.rpm`);

    const launcherUrl = resolvePackageDownloadUrl(`artcraft-launcher-${launcherVersion}-linux-riscv64.deb`);
    expect(launcherUrl).toBe(`${launcherBase}/artcraft-launcher-${launcherVersion}-linux-riscv64.deb`);

    const archUrl = resolvePackageDownloadUrl(`pdfcraft-${pdf}-linux-x86_64.pkg.tar.zst`);
    expect(resolvePackageDownloadUrl(`pdfcraft-${pdf}-linux-x86_64.pkg.tar.zst.sig`)).toBe(
      `${pdfBase}/pdfcraft-${pdf}-linux-x86_64.pkg.tar.zst.sig`,
    );
    expect(resolvePackageDownloadUrl(`pdfcraft-${pdf}-linux-x86_64.rpm.asc`)).toBe(
      `${pdfBase}/pdfcraft-${pdf}-linux-x86_64.rpm.asc`,
    );
    // Signatures are only served for packages the catalog publishes.
    expect(resolvePackageDownloadUrl(`unknown-${pdf}-linux-x86_64.pkg.tar.zst.sig`)).toBeNull();
    expect(archUrl).toBe(`${pdfBase}/pdfcraft-${pdf}-linux-x86_64.pkg.tar.zst`);

    const archArmUrl = resolvePackageDownloadUrl(`artcraft-launcher-${launcherVersion}-linux-aarch64.pkg.tar.zst`);
    expect(archArmUrl).toBe(`${launcherBase}/artcraft-launcher-${launcherVersion}-linux-aarch64.pkg.tar.zst`);

    const craftAnythingFile = `craft-anything-${craftAnything.version}-linux-x86_64.deb`;
    expect(resolvePackageDownloadUrl(craftAnythingFile)).toBe(
      `https://github.com/storytold/craft-anything/releases/download/v${craftAnything.version}/${craftAnythingFile}`,
    );

    // Uncatalogued versions or files must return null so route returns 404
    expect(resolvePackageDownloadUrl("pdfcraft-9.9.9-linux-x86_64.deb")).toBeNull();
    expect(resolvePackageDownloadUrl("unknown-app-1.0.0-linux-x86_64.rpm")).toBeNull();
    expect(resolvePackageDownloadUrl("some-random-file.tar.gz")).toBeNull();
  });

  test("generates valid APT Packages stanzas", async () => {
    const amd64Packages = await generateAptPackages("amd64");
    expect(amd64Packages).toContain("Package: pdfcraft");
    expect(amd64Packages).toContain("Architecture: amd64");
    expect(amd64Packages).toContain(`Filename: pool/main/pdfcraft/pdfcraft-${pdf}-linux-x86_64.deb`);
    const sums = await (await fetch(pdfcraft.download("SHA256SUMS.txt"))).text();
    const debSha = sums.match(new RegExp(`^([0-9a-f]{64})\\s+\\*?pdfcraft-${pdf}-linux-x86_64\\.deb$`, "m"))?.[1];
    expect(debSha).toBeDefined();
    expect(amd64Packages).toContain(`SHA256: ${debSha}`);

    const arm64Packages = await generateAptPackages("arm64");
    expect(arm64Packages).toContain("Architecture: arm64");
    expect(arm64Packages).toContain(`Filename: pool/main/pdfcraft/pdfcraft-${pdf}-linux-aarch64.deb`);

    const riscv64Packages = await generateAptPackages("riscv64");
    expect(riscv64Packages).toContain("Architecture: riscv64");
    expect(riscv64Packages).toContain(`Filename: pool/main/pdfcraft/pdfcraft-${pdf}-linux-riscv64.deb`);
    expect(riscv64Packages).toContain("Package: artcraft-launcher");
  });

  test("generates valid APT Release index with hashes", async () => {
    const release = await generateAptRelease();
    expect(release).toContain("Origin: ArtCraft");
    expect(release).toContain("Suite: stable");
    expect(release).toContain("Architectures: amd64 arm64 riscv64");
    expect(release).toContain("main/binary-amd64/Packages");
    expect(release).toContain("main/binary-amd64/Packages.gz");
  });

  test("generates APT sources configs with trusted=yes", async () => {
    const list = await generateAptSourcesList("https://getartcraft.com");
    expect(list).toBe("deb [trusted=yes] https://getartcraft.com/apt stable main\n");

    const sources = await generateAptSourcesDeb822("https://getartcraft.com");
    expect(sources).toContain("Types: deb");
    expect(sources).toContain("URIs: https://getartcraft.com/apt");
    expect(sources).toContain("Architectures: amd64 arm64 riscv64");
    expect(sources).toContain("Trusted: yes");
  });

  test("generates valid RPM primary metadata XML with requires and dependencies", async () => {
    const primaryXml = await generateRpmPrimaryXml();
    expect(primaryXml).toContain('<metadata xmlns="http://linux.duke.edu/metadata/common"');
    expect(primaryXml).toContain("<name>pdfcraft</name>");
    expect(primaryXml).toContain("<arch>x86_64</arch>");
    expect(primaryXml).toContain(`<location href="packages/pdfcraft-${pdf}-linux-x86_64.rpm"/>`);
    expect(primaryXml).toContain("<arch>aarch64</arch>");
    expect(primaryXml).toContain("<arch>riscv64</arch>");
    expect(primaryXml).toContain("<name>artcraft-launcher</name>");

    // Check declared RPM requirements
    expect(primaryXml).toContain('<rpm:entry name="libxkbcommon.so.0()(64bit)"/>');
    expect(primaryXml).toContain('<rpm:entry name="(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))"/>');
    expect(primaryXml).toContain('<rpm:entry name="glibc" flags="GE" epoch="0" ver="2.35"/>');

    const x86Xml = await generateRpmPrimaryXml(RPM_REPO_TIMESTAMP, "x86_64");
    expect(x86Xml).toContain("<arch>x86_64</arch>");
    expect(x86Xml).not.toContain("<arch>aarch64</arch>");
    expect(x86Xml).not.toContain("<arch>riscv64</arch>");

    const riscvXml = await generateRpmPrimaryXml(RPM_REPO_TIMESTAMP, "riscv64");
    expect(riscvXml).toContain("<arch>riscv64</arch>");
    expect(riscvXml).not.toContain("<arch>x86_64</arch>");
  });

  test("generates valid, stable repomd.xml matching primary.xml.gz checksums", async () => {
    const repomd = await getRpmRepomdXml();
    expect(repomd).toContain('<repomd xmlns="http://linux.duke.edu/metadata/repo">');
    expect(repomd).toContain('<data type="primary">');
    expect(repomd).toContain('<location href="repodata/primary.xml.gz"/>');

    const gzBuffer = await getRpmPrimaryGz();
    const primaryXml = await getRpmPrimaryXml();
    const gzSha = createHash("sha256").update(gzBuffer).digest("hex");
    const openSha = createHash("sha256").update(Buffer.from(primaryXml, "utf-8")).digest("hex");

    expect(repomd).toContain(`<checksum type="sha256">${gzSha}</checksum>`);
    expect(repomd).toContain(`<open-checksum type="sha256">${openSha}</open-checksum>`);
  });

  test("generates RPM repo config with gpgcheck=0", async () => {
    const repo = await generateRpmRepoConfig("https://getartcraft.com");
    expect(repo).toContain("[artcraft]");
    expect(repo).toContain("baseurl=https://getartcraft.com/rpm");
    expect(repo).toContain("gpgcheck=0");
  });

  test("validates distribution and architecture guard sets", () => {
    expect(VALID_APT_DISTRIBUTIONS.has("stable")).toBe(true);
    expect(VALID_APT_DISTRIBUTIONS.has("unknown")).toBe(false);
    expect(VALID_APT_COMPONENTS.has("main")).toBe(true);
    expect(VALID_APT_COMPONENTS.has("contrib")).toBe(false);
    expect(VALID_APT_ARCHES.has("amd64")).toBe(true);
    expect(VALID_APT_ARCHES.has("arm64")).toBe(true);
    expect(VALID_APT_ARCHES.has("riscv64")).toBe(true);
    expect(VALID_APT_ARCHES.has("i386")).toBe(false);
    expect(VALID_RPM_ARCHES.has("x86_64")).toBe(true);
    expect(VALID_RPM_ARCHES.has("aarch64")).toBe(true);
    expect(VALID_RPM_ARCHES.has("riscv64")).toBe(true);
    expect(VALID_RPM_ARCHES.has("mips")).toBe(false);
    expect(VALID_PACMAN_ARCHES.has("x86_64")).toBe(true);
    expect(VALID_PACMAN_ARCHES.has("aarch64")).toBe(true);
    expect(VALID_PACMAN_ARCHES.has("riscv64")).toBe(false);
  });

  test("escapes XML entities safely", () => {
    expect(escapeXml('foo & "bar" <baz>')).toBe("foo &amp; &quot;bar&quot; &lt;baz&gt;");
  });

  test("keeps valid multiline armor intact and only unescapes single-line keys", () => {
    // A `Comment:` header may contain backslashes; rewriting them would corrupt the armor and
    // silently disable signing for an otherwise valid key.
    const armored = [
      "-----BEGIN PGP PRIVATE KEY BLOCK-----",
      "Comment: exported from the release host",
      "",
      "lQOYBGexampleKeyMaterial",
      "-----END PGP PRIVATE KEY BLOCK-----",
    ].join("\n");
    expect(normalizeArmoredKey(armored)).toBe(armored);

    const windowsPathComment = armored.replace(
      "Comment: exported from the release host",
      "Comment: keys live at C:\\Users\\release",
    );
    expect(normalizeArmoredKey(windowsPathComment)).toBe(windowsPathComment);

    // The same block as one line of JSON / .env / `netlify env:set` escapes.
    expect(normalizeArmoredKey(`"${armored.replaceAll("\n", "\\n")}"`)).toBe(armored);
    // A JSON value stored inside another JSON value doubles every backslash.
    expect(normalizeArmoredKey(`"${armored.replaceAll("\n", "\\\\n")}"`)).toBe(armored);

    // Base64 travels as a single line, wrapped or not, and may lose its padding on the way.
    const base64 = Buffer.from(armored, "utf-8").toString("base64");
    expect(normalizeArmoredKey(base64)).toBe(armored);
    expect(normalizeArmoredKey(base64.replace(/=+$/, ""))).toBe(armored);
    expect(normalizeArmoredKey(base64.replace(/(.{76})/g, "$1\n"))).toBe(armored);
    // Base64 of an escaped single line still needs the newlines put back after decoding.
    const escapedThenEncoded = Buffer.from(armored.replaceAll("\n", "\\n"), "utf-8").toString("base64");
    expect(normalizeArmoredKey(escapedThenEncoded)).toBe(armored);
  });

  test("withholds Arch package signatures when the deployment publishes no key", async () => {
    // pacman has no key to verify against here, so serving the signature would only break the
    // install; the config for this deployment does not ask for signatures either.
    const filename = `pdfcraft-${pdf}-linux-x86_64.pkg.tar.zst.sig`;
    const response = await getArchPackage(
      new Request(`https://getartcraft.com/arch/x86_64/${filename}`),
      { params: Promise.resolve({ arch: "x86_64", filename }) },
    );
    expect(response.status).toBe(404);
  });

  test("uses trusted origins for repository configuration", () => {
    const directReq = new Request("http://127.0.0.1:4210/rpm/artcraft.repo");
    expect(getRequestBaseUrl(directReq)).toBe("http://127.0.0.1:4210");

    const proxiedReq = new Request("http://localhost:3000/apt/artcraft.sources", {
      headers: {
        "x-forwarded-host": "getartcraft.com",
        "x-forwarded-proto": "https",
      },
    });
    expect(getRequestBaseUrl(proxiedReq)).toBe("http://localhost:3000");

    // The local test proxy in scripts/test-distro-repos.sh forwards to the Next server; Flatpak
    // must receive the proxy's own address, since the Next server drops cleartext HTTP/2.
    const forwarderReq = new Request("http://localhost:4210/artcraft-oci.flatpakrepo", {
      headers: {
        "x-forwarded-host": "127.0.0.1:4214",
        "x-forwarded-proto": "http",
      },
    });
    expect(getRequestBaseUrl(forwarderReq)).toBe("http://127.0.0.1:4214");

    const hostHeaderReq = new Request("http://0.0.0.0:4210/artcraft-oci.flatpakrepo", {
      headers: {
        host: "artcraft.internal:4210",
      },
    });
    expect(getRequestBaseUrl(hostHeaderReq)).toBe("https://getartcraft.com");
    const preview = new Request("https://deploy-preview-204--artcraft-website-next.netlify.app/apt/artcraft.sources", {
      headers: { "x-forwarded-host": "attacker.example" },
    });
    expect(getRequestBaseUrl(preview)).toBe("https://deploy-preview-204--artcraft-website-next.netlify.app");
  });

  test("mirrors the package's .PKGINFO provides and optdepends in the pacman database", () => {
    const app = {
      slug: "examplecraft",
      name: "examplecraft",
      version: "1.2.3",
      description: "Example",
      section: "graphics",
      debAssets: [],
      rpmAssets: [],
      archAssets: [],
    };
    const asset = {
      arch: "x86_64" as const,
      filename: "examplecraft-1.2.3-linux-x86_64.pkg.tar.zst",
      size: 10,
      sha256: "a".repeat(64),
      installedSize: 20,
      depends: ["glibc"],
      provides: ["examplecraft=1.2.3-1"],
      optdepends: ["cups: printing support"],
      signed: true,
    };

    const mirrored = generatePacmanDesc(app, asset);
    expect(mirrored).toContain("%PROVIDES%\nexamplecraft=1.2.3-1\n");
    expect(mirrored).toContain("%OPTDEPENDS%\ncups: printing support\n");
    expect(mirrored).not.toContain("alsa-lib");

    // Only when .PKGINFO could not be read does the shared builder's list stand in.
    const fallback = generatePacmanDesc(app, { ...asset, provides: [], optdepends: [] });
    expect(fallback).toContain("%PROVIDES%\nexamplecraft=1.2.3-1\n");
    expect(fallback).toContain("alsa-lib: sound support");
  });

  test("provides metadata and catalog for /linux setup page", async () => {
    const apps = await getLinuxRepoApps();
    // The launcher plus every app that publishes Linux packages, MaterialCraft included.
    expect(apps).toHaveLength(17);
    const pdfcraftApp = apps.find((app) => app.slug === "pdfcraft");
    expect(pdfcraftApp).toBeDefined();
    expect(pdfcraftApp?.version).toBe(pdf);
    expect(pdfcraftApp?.debAssets).toHaveLength(pdfcraft.count(".deb"));
    expect(pdfcraftApp?.rpmAssets).toHaveLength(pdfcraft.count(".rpm"));
    expect(pdfcraftApp?.archAssets).toHaveLength(pdfcraft.count(".pkg.tar.zst"));

    const launcher = apps.find((app) => app.slug === "artcraft-launcher");
    expect(launcher).toBeDefined();
    expect(launcher?.debAssets).toHaveLength(3);
    expect(launcher?.rpmAssets).toHaveLength(3);
    expect(launcher?.archAssets).toHaveLength(2);

    const texturecraft = apps.find((app) => app.slug === "texturecraft");
    expect(texturecraft).toBeDefined();
    expect(texturecraft?.version).toBe(catalogRelease("texturecraft").version);

    const craftAnythingApp = apps.find((app) => app.slug === "craft-anything");
    expect(craftAnythingApp).toBeDefined();
    expect(craftAnythingApp?.debAssets).toHaveLength(craftAnything.count(".deb"));
    expect(craftAnythingApp?.rpmAssets).toHaveLength(craftAnything.count(".rpm"));
    expect(craftAnythingApp?.archAssets).toHaveLength(craftAnything.count(".pkg.tar.zst"));
  });

  test("finds .PKGINFO that comes after a large payload in a package tar", () => {
    // designcraft ships its two binaries before .PKGINFO, so a 128KB Range probe cannot see it and
    // pacman rejects the package when the sync database guesses the install size or depends.
    const pkgInfo = `# Generated by makepkg
pkgname = ai.storyteller.designcraft
pkgver = 0.17.0-1
arch = x86_64
size = 241103068
depend = glibc
depend = gcc-libs
depend = vulkan-icd-loader
provides = designcraft=0.17.0-1
optdepend = alsa-lib: sound support
optdepend = cups: printing support

`;
    const tar = buildTar([
      { name: "usr/", type: "dir" },
      { name: "usr/bin/designcraft", type: "file", content: "x".repeat(300_000) },
      { name: "usr/bin/designcraft-cli", type: "file", content: "y".repeat(200_000) },
      { name: ".PKGINFO", type: "file", content: pkgInfo },
    ]);

    // Feed the archive in small chunks the way a download stream arrives.
    const scanner = createPkgInfoScanner();
    let info: ReturnType<typeof scanner.push> = null;
    for (let offset = 0; offset < tar.length && !info; offset += 4096) {
      info = scanner.push(tar.subarray(offset, offset + 4096));
    }

    expect(info).toEqual({
      installedSize: 241103068,
      depends: ["glibc", "gcc-libs", "vulkan-icd-loader"],
      provides: ["designcraft=0.17.0-1"],
      optdepends: ["alsa-lib: sound support", "cups: printing support"],
    });
  });

  test("generates valid pacman sync database and repo config", async () => {
    const repoConfig = await generatePacmanRepoConfig("https://getartcraft.com");
    expect(repoConfig).toContain("[artcraft]");
    expect(repoConfig).toContain("Server = https://getartcraft.com/arch/$arch");
    expect(repoConfig).toContain("SigLevel = Optional TrustAll");

    const x86Db = await generatePacmanDb("x86_64");
    expect(x86Db.length).toBeGreaterThan(100);

    const armDb = await generatePacmanDb("aarch64");
    expect(armDb.length).toBeGreaterThan(100);
  });

  test("keeps all distro tabs reachable on mobile and shows the supported RHEL baseline", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/linux");

    const repoTablist = page.getByRole("tablist").filter({
      has: page.getByRole("tab", { name: "Flatpak (OCI)" }),
    });
    const dimensions = await repoTablist.evaluate((element) => ({
      clientWidth: element.clientWidth,
      scrollWidth: element.scrollWidth,
    }));
    expect(dimensions.scrollWidth).toBeGreaterThan(dimensions.clientWidth);

    await page.getByRole("tab", { name: "Arch Linux" }).click();
    await expect(page.getByRole("tab", { name: "Arch Linux" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByText(/Include = \/etc\/pacman\.d\/artcraft\.conf/)).toBeVisible();

    await page.getByRole("tab", { name: "Fedora / RHEL" }).click();
    await expect(page.getByText("Fedora 40+, RHEL 10+, CentOS Stream, Rocky Linux, AlmaLinux")).toBeVisible();
  });

  test("verifies download URLs and digests for all manifest package assets", async () => {
    const apps = await getLinuxRepoApps();
    for (const app of apps) {
      for (const deb of app.debAssets) {
        expect(deb.size).toBeGreaterThan(10_000_000);
        expect(deb.sha256).toMatch(/^[0-9a-f]{64}$/);
        expect(deb.sha256).not.toMatch(/^0+$/);
        expect(deb.filename).toMatch(/\.deb$/);
        const url = resolvePackageDownloadUrl(deb.filename);
        expect(url).not.toBeNull();
        expect(url).toContain(app.version);
        expect(url).toContain(deb.filename);
      }
      for (const rpm of app.rpmAssets) {
        expect(rpm.size).toBeGreaterThan(10_000_000);
        expect(rpm.sha256).toMatch(/^[0-9a-f]{64}$/);
        expect(rpm.sha256).not.toMatch(/^0+$/);
        expect(rpm.filename).toMatch(/\.rpm$/);
        const url = resolvePackageDownloadUrl(rpm.filename);
        expect(url).not.toBeNull();
        expect(url).toContain(app.version);
        expect(url).toContain(rpm.filename);
      }
      for (const arch of app.archAssets) {
        expect(arch.size).toBeGreaterThan(5_000_000);
        expect(arch.sha256).toMatch(/^[0-9a-f]{64}$/);
        expect(arch.sha256).not.toMatch(/^0+$/);
        expect(arch.filename).toMatch(/\.pkg\.tar\.zst$/);
        const url = resolvePackageDownloadUrl(arch.filename);
        expect(url).not.toBeNull();
        expect(url).toContain(app.version);
        expect(url).toContain(arch.filename);
      }
    }
  });
});

// With LINUX_SIGNING_KEY configured the repositories publish signed metadata and the client
// configs stop trusting blindly. The signatures are checked with openpgp.js itself; the install
// paths themselves were exercised against apt, dnf and pacman containers.
test.describe("signed repositories", () => {
  let secretKey: string;
  let publicKeyArmored: string;

  test.beforeAll(async () => {
    const generated = await openpgp.generateKey({
      type: "curve25519",
      userIDs: [{ name: "ArtCraft Test", email: "test@example.com" }],
      format: "armored",
    });
    secretKey = generated.privateKey;
    publicKeyArmored = generated.publicKey;
  });

  test.beforeEach(() => {
    process.env.LINUX_SIGNING_KEY = secretKey;
  });

  test.afterEach(() => {
    delete process.env.LINUX_SIGNING_KEY;
  });

  test("points APT, DNF and pacman at the published key instead of trusting blindly", async () => {
    expect(await generateAptSourcesList("https://getartcraft.com")).toBe(
      "deb [signed-by=/etc/apt/keyrings/artcraft.gpg] https://getartcraft.com/apt stable main\n",
    );

    const sources = await generateAptSourcesDeb822("https://getartcraft.com");
    expect(sources).toContain("Signed-By: /etc/apt/keyrings/artcraft.gpg");
    expect(sources).not.toContain("Trusted: yes");

    const repo = await generateRpmRepoConfig("https://getartcraft.com");
    expect(repo).toContain("repo_gpgcheck=1");
    expect(repo).toContain("gpgcheck=1");
    expect(repo).not.toContain("gpgcheck=0");
    expect(repo).toContain("gpgkey=https://getartcraft.com/keys/artcraft.asc");

    const pacmanConfig = await generatePacmanRepoConfig("https://getartcraft.com");
    // Both the sync database and every downloaded package must be signed.
    expect(pacmanConfig).toContain("SigLevel = Required DatabaseRequired");
    expect(pacmanConfig).not.toContain("TrustAll");
  });

  test("withholds packages whose release publishes no signature", async () => {
    const apps = await getLinuxRepoApps({ rpm: true, arch: true });

    // Everything the repository advertises has to be verifiable. A release that predates signing,
    // like craft-anything 0.3.0, is left out until it publishes signatures with the next release.
    for (const app of apps) {
      expect(app.rpmAssets.every((asset) => asset.signed)).toBe(true);
      expect(app.archAssets.every((asset) => asset.signed)).toBe(true);
    }

    const signed = { filename: "signedcraft-1.0.0-linux-x86_64.rpm", signed: true };
    const unsigned = { filename: "unsignedcraft-1.0.0-linux-x86_64.rpm", signed: false };
    expect(await withholdUnsignedPackages([signed, unsigned])).toEqual([signed]);

    // With no key on the deployment there is nothing to verify against, so nothing is withheld.
    delete process.env.LINUX_SIGNING_KEY;
    expect(await withholdUnsignedPackages([signed, unsigned])).toEqual([signed, unsigned]);
  });

  test("redirects arch package signature downloads to the release asset", async () => {
    const filename = `pdfcraft-${pdf}-linux-x86_64.pkg.tar.zst.sig`;
    const response = await getArchPackage(
      new Request(`https://getartcraft.com/arch/x86_64/${filename}`),
      { params: Promise.resolve({ arch: "x86_64", filename }) },
    );

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toBe(
      `https://github.com/storytold/pdfcraft/releases/download/v${pdf}/${filename}`,
    );

    // Uncatalogued versions have no release asset to redirect to.
    const unknown = "pdfcraft-9.9.9-linux-x86_64.pkg.tar.zst.sig";
    const missing = await getArchPackage(
      new Request(`https://getartcraft.com/arch/x86_64/${unknown}`),
      { params: Promise.resolve({ arch: "x86_64", filename: unknown }) },
    );
    expect(missing.status).toBe(404);
  });

  test("signs the apt Release so apt accepts both Release.gpg and InRelease", async () => {
    const release = await generateAptRelease();
    const publicKey = await openpgp.readKey({ armoredKey: publicKeyArmored });
    const message = await openpgp.createMessage({ binary: Buffer.from(release, "utf-8") });

    const detached = await signDetachedArmored(release);
    expect(detached).toContain("-----BEGIN PGP SIGNATURE-----");
    const detachedResult = await openpgp.verify({
      message,
      signature: await openpgp.readSignature({ armoredSignature: detached! }),
      verificationKeys: publicKey,
    });
    await detachedResult.signatures[0].verified;

    const cleartext = await signCleartext(release);
    expect(cleartext).toContain("-----BEGIN PGP SIGNED MESSAGE-----");
    const cleartextResult = await openpgp.verify({
      message: await openpgp.readCleartextMessage({ cleartextMessage: cleartext! }),
      verificationKeys: publicKey,
    });
    await cleartextResult.signatures[0].verified;
  });

  test("signs repomd.xml for dnf", async () => {
    const repomd = await getRpmRepomdXml();
    const signature = await signDetachedArmored(repomd);
    const result = await openpgp.verify({
      message: await openpgp.createMessage({ binary: Buffer.from(repomd, "utf-8") }),
      signature: await openpgp.readSignature({ armoredSignature: signature! }),
      verificationKeys: await openpgp.readKey({ armoredKey: publicKeyArmored }),
    });
    await result.signatures[0].verified;
  });

  test("signs the pacman database in binary form", async () => {
    const db = await generatePacmanDb("x86_64");
    const signature = await signDetachedBinary(new Uint8Array(db));
    expect(signature).not.toBeNull();
    // A binary signature starts with an OpenPGP packet, not with armor text.
    const firstByte = Buffer.from(signature!).subarray(0, 1)[0];
    expect(firstByte & 0x40).toBe(0x40); // new-format packet header
    expect(firstByte & 0x3f).toBe(2); // signature packet


    const result = await openpgp.verify({
      message: await openpgp.createMessage({ binary: new Uint8Array(db) }),
      signature: await openpgp.readSignature({ binarySignature: signature! }),
      verificationKeys: await openpgp.readKey({ armoredKey: publicKeyArmored }),
    });
    await result.signatures[0].verified;
  });

  test("publishes the fingerprint with the public key", async () => {
    const key = await getSigningPublicKey();
    expect(key?.armored).toContain("-----BEGIN PGP PUBLIC KEY BLOCK-----");
    const publicKey = await openpgp.readKey({ armoredKey: publicKeyArmored });
    expect(key?.fingerprint).toBe(publicKey.getFingerprint().toUpperCase());

    // The binary route serves this directly, so it must be the public half, not the private key.
    const binaryKey = await openpgp.readKey({ binaryKey: key!.binary });
    expect(binaryKey.isPrivate()).toBe(false);
    expect(binaryKey.getFingerprint().toUpperCase()).toBe(publicKey.getFingerprint().toUpperCase());
  });

  test("falls back to unsigned metadata when no key is configured", async () => {
    delete process.env.LINUX_SIGNING_KEY;
    expect(await isRepoSigningConfigured()).toBe(false);
    expect(await signDetachedArmored("Release")).toBeNull();
    expect(await getSigningPublicKey()).toBeNull();
    expect(await generateAptSourcesDeb822("https://getartcraft.com")).toContain("Trusted: yes");
  });

  test("signature routes return 404 before generating metadata without a key", async () => {
    delete process.env.LINUX_SIGNING_KEY;
    const aptProps = { params: Promise.resolve({ distribution: "stable" }) };
    const rpmProps = { params: Promise.resolve({ arch: "x86_64" }) };
    const archProps = { params: Promise.resolve({ arch: "x86_64" }) };

    expect((await getInRelease(new Request("https://getartcraft.com/apt/dists/stable/InRelease"), aptProps)).status).toBe(404);
    expect((await getReleaseGpg(new Request("https://getartcraft.com/apt/dists/stable/Release.gpg"), aptProps)).status).toBe(404);
    expect((await getRpmSignature(new Request("https://getartcraft.com/rpm/x86_64/repodata/repomd.xml.asc"), rpmProps)).status).toBe(404);
    expect((await getArchSignature(new Request("https://getartcraft.com/arch/x86_64/artcraft.db.sig"), archProps)).status).toBe(404);
  });

  test("serves verifiable signatures and the public key from the routes", async () => {
    const publicKey = await openpgp.readKey({ armoredKey: publicKeyArmored });
    const fingerprint = publicKey.getFingerprint().toUpperCase();

    const inRelease = await getInRelease(
      new Request("https://getartcraft.com/apt/dists/stable/InRelease"),
      { params: Promise.resolve({ distribution: "stable" }) },
    );
    expect(inRelease.status).toBe(200);
    const cleartext = await openpgp.verify({
      message: await openpgp.readCleartextMessage({ cleartextMessage: await inRelease.text() }),
      verificationKeys: publicKey,
    });
    await cleartext.signatures[0].verified;

    const release = await generateAptRelease();
    const releaseGpg = await getReleaseGpg(
      new Request("https://getartcraft.com/apt/dists/stable/Release.gpg"),
      { params: Promise.resolve({ distribution: "stable" }) },
    );
    expect(releaseGpg.status).toBe(200);
    const detached = await openpgp.verify({
      message: await openpgp.createMessage({ binary: Buffer.from(release, "utf-8") }),
      signature: await openpgp.readSignature({ armoredSignature: await releaseGpg.text() }),
      verificationKeys: publicKey,
    });
    await detached.signatures[0].verified;

    for (const arch of VALID_RPM_ARCHES) {
      const response = await getRpmSignature(
        new Request(`https://getartcraft.com/rpm/${arch}/repodata/repomd.xml.asc`),
        { params: Promise.resolve({ arch }) },
      );
      expect(response.status).toBe(200);
      const verified = await openpgp.verify({
        message: await openpgp.createMessage({ binary: Buffer.from(await getRpmRepomdXml(arch), "utf-8") }),
        signature: await openpgp.readSignature({ armoredSignature: await response.text() }),
        verificationKeys: publicKey,
      });
      await verified.signatures[0].verified;
    }

    for (const arch of VALID_PACMAN_ARCHES) {
      const response = await getArchSignature(
        new Request(`https://getartcraft.com/arch/${arch}/artcraft.db.sig`),
        { params: Promise.resolve({ arch }) },
      );
      expect(response.status).toBe(200);
      const verified = await openpgp.verify({
        message: await openpgp.createMessage({ binary: new Uint8Array(await generatePacmanDb(arch)) }),
        signature: await openpgp.readSignature({ binarySignature: new Uint8Array(await response.arrayBuffer()) }),
        verificationKeys: publicKey,
      });
      await verified.signatures[0].verified;
    }

    const armoredKey = await openpgp.readKey({ armoredKey: await (await getKeyAsc()).text() });
    expect(armoredKey.getFingerprint().toUpperCase()).toBe(fingerprint);

    const binaryKey = await openpgp.readKey({
      binaryKey: new Uint8Array(await (await getKeyGpg()).arrayBuffer()),
    });
    expect(binaryKey.isPrivate()).toBe(false);
    expect(binaryKey.getFingerprint().toUpperCase()).toBe(fingerprint);
  });
});
