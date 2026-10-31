import { expect, test } from "@playwright/test";
import {
  generateAptPackages,
  generateAptRelease,
  generateAptSourcesList,
  generateAptSourcesDeb822,
  generateRpmPrimaryXml,
  generateRpmRepomdXml,
  generateRpmRepoConfig,
  getRpmPrimaryXml,
  getRpmPrimaryGz,
  getRpmRepomdXml,
  resolvePackageDownloadUrl,
  RPM_REPO_TIMESTAMP,
  LINUX_REPO_APPS,
  VALID_APT_DISTRIBUTIONS,
  VALID_APT_COMPONENTS,
  VALID_APT_ARCHES,
  VALID_RPM_ARCHES,
  escapeXml,
  getRequestBaseUrl,
} from "../src/lib/linux-repos";
import { createHash } from "node:crypto";

test.describe("Linux Package Repositories (APT & RPM)", () => {
  test("resolves release download URLs only for catalogued package assets", () => {
    const debUrl = resolvePackageDownloadUrl("pdfcraft-0.16.1-linux-x86_64.deb");
    expect(debUrl).toBe("https://github.com/storytold/pdfcraft/releases/download/v0.16.1/pdfcraft-0.16.1-linux-x86_64.deb");

    const rpmUrl = resolvePackageDownloadUrl("pdfcraft-0.16.1-linux-x86_64.rpm");
    expect(rpmUrl).toBe("https://github.com/storytold/pdfcraft/releases/download/v0.16.1/pdfcraft-0.16.1-linux-x86_64.rpm");

    const launcherUrl = resolvePackageDownloadUrl("artcraft-launcher-0.11.1-linux-riscv64.deb");
    expect(launcherUrl).toBe("https://github.com/storytold/craft-launcher/releases/download/v0.11.1/artcraft-launcher-0.11.1-linux-riscv64.deb");

    // Uncatalogued versions or files must return null so route returns 404
    expect(resolvePackageDownloadUrl("pdfcraft-9.9.9-linux-x86_64.deb")).toBeNull();
    expect(resolvePackageDownloadUrl("unknown-app-1.0.0-linux-x86_64.rpm")).toBeNull();
    expect(resolvePackageDownloadUrl("some-random-file.tar.gz")).toBeNull();
  });

  test("generates valid APT Packages stanzas", () => {
    const amd64Packages = generateAptPackages("amd64");
    expect(amd64Packages).toContain("Package: pdfcraft");
    expect(amd64Packages).toContain("Architecture: amd64");
    expect(amd64Packages).toContain("Filename: pool/main/pdfcraft/pdfcraft-0.16.1-linux-x86_64.deb");
    expect(amd64Packages).toContain("SHA256: fb71f9b366214c8b0dafe01afba80d5280d554a7683ab7756f601a170668823b");

    const arm64Packages = generateAptPackages("arm64");
    expect(arm64Packages).toContain("Architecture: arm64");
    expect(arm64Packages).toContain("Filename: pool/main/pdfcraft/pdfcraft-0.16.1-linux-aarch64.deb");

    const riscv64Packages = generateAptPackages("riscv64");
    expect(riscv64Packages).toContain("Architecture: riscv64");
    expect(riscv64Packages).toContain("Filename: pool/main/pdfcraft/pdfcraft-0.16.1-linux-riscv64.deb");
    expect(riscv64Packages).toContain("Package: artcraft-launcher");
  });

  test("generates valid APT Release index with hashes", () => {
    const release = generateAptRelease();
    expect(release).toContain("Origin: ArtCraft");
    expect(release).toContain("Suite: stable");
    expect(release).toContain("Architectures: amd64 arm64 riscv64");
    expect(release).toContain("main/binary-amd64/Packages");
    expect(release).toContain("main/binary-amd64/Packages.gz");
  });

  test("generates APT sources configs with trusted=yes", () => {
    const list = generateAptSourcesList("https://getartcraft.com");
    expect(list).toBe("deb [trusted=yes] https://getartcraft.com/apt stable main\n");

    const sources = generateAptSourcesDeb822("https://getartcraft.com");
    expect(sources).toContain("Types: deb");
    expect(sources).toContain("URIs: https://getartcraft.com/apt");
    expect(sources).toContain("Architectures: amd64 arm64 riscv64");
    expect(sources).toContain("Trusted: yes");
  });

  test("generates valid RPM primary metadata XML with requires and dependencies", () => {
    const primaryXml = generateRpmPrimaryXml();
    expect(primaryXml).toContain('<metadata xmlns="http://linux.duke.edu/metadata/common"');
    expect(primaryXml).toContain("<name>pdfcraft</name>");
    expect(primaryXml).toContain("<arch>x86_64</arch>");
    expect(primaryXml).toContain('<location href="packages/pdfcraft-0.16.1-linux-x86_64.rpm"/>');
    expect(primaryXml).toContain("<arch>aarch64</arch>");
    expect(primaryXml).toContain("<arch>riscv64</arch>");
    expect(primaryXml).toContain("<name>artcraft-launcher</name>");

    // Check declared RPM requirements
    expect(primaryXml).toContain('<rpm:entry name="libxkbcommon.so.0()(64bit)"/>');
    expect(primaryXml).toContain('<rpm:entry name="(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))"/>');
    expect(primaryXml).toContain('<rpm:entry name="glibc" flags="GE" epoch="0" ver="2.35"/>');

    const x86Xml = generateRpmPrimaryXml(RPM_REPO_TIMESTAMP, "x86_64");
    expect(x86Xml).toContain("<arch>x86_64</arch>");
    expect(x86Xml).not.toContain("<arch>aarch64</arch>");
    expect(x86Xml).not.toContain("<arch>riscv64</arch>");

    const riscvXml = generateRpmPrimaryXml(RPM_REPO_TIMESTAMP, "riscv64");
    expect(riscvXml).toContain("<arch>riscv64</arch>");
    expect(riscvXml).not.toContain("<arch>x86_64</arch>");
  });

  test("generates valid, stable repomd.xml matching primary.xml.gz checksums", () => {
    const repomd = getRpmRepomdXml();
    expect(repomd).toContain('<repomd xmlns="http://linux.duke.edu/metadata/repo">');
    expect(repomd).toContain('<data type="primary">');
    expect(repomd).toContain('<location href="repodata/primary.xml.gz"/>');

    const gzBuffer = getRpmPrimaryGz();
    const primaryXml = getRpmPrimaryXml();
    const gzSha = createHash("sha256").update(gzBuffer).digest("hex");
    const openSha = createHash("sha256").update(Buffer.from(primaryXml, "utf-8")).digest("hex");

    expect(repomd).toContain(`<checksum type="sha256">${gzSha}</checksum>`);
    expect(repomd).toContain(`<open-checksum type="sha256">${openSha}</open-checksum>`);
  });

  test("generates RPM repo config with gpgcheck=0", () => {
    const repo = generateRpmRepoConfig("https://getartcraft.com");
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
  });

  test("escapes XML entities safely", () => {
    expect(escapeXml('foo & "bar" <baz>')).toBe("foo &amp; &quot;bar&quot; &lt;baz&gt;");
  });

  test("extracts request base URL supporting forwarded headers", () => {
    const directReq = new Request("http://127.0.0.1:4210/rpm/artcraft.repo");
    expect(getRequestBaseUrl(directReq)).toBe("http://127.0.0.1:4210");

    const proxiedReq = new Request("http://localhost:3000/apt/artcraft.sources", {
      headers: {
        "x-forwarded-host": "getartcraft.com",
        "x-forwarded-proto": "https",
      },
    });
    expect(getRequestBaseUrl(proxiedReq)).toBe("https://getartcraft.com");

    const hostHeaderReq = new Request("http://0.0.0.0:4210/artcraft-oci.flatpakrepo", {
      headers: {
        host: "artcraft.internal:4210",
      },
    });
    expect(getRequestBaseUrl(hostHeaderReq)).toBe("http://artcraft.internal:4210");
  });

  test("provides metadata and catalog for /linux setup page", () => {
    expect(LINUX_REPO_APPS.length).toBeGreaterThan(0);
    const pdfcraft = LINUX_REPO_APPS.find((app) => app.slug === "pdfcraft");
    expect(pdfcraft).toBeDefined();
    expect(pdfcraft?.debAssets.length).toBe(3);
    expect(pdfcraft?.rpmAssets.length).toBe(3);

    const launcher = LINUX_REPO_APPS.find((app) => app.slug === "artcraft-launcher");
    expect(launcher).toBeDefined();
    expect(launcher?.debAssets.length).toBe(3);
    expect(launcher?.rpmAssets.length).toBe(3);
  });
});
