import { expect, test } from "@playwright/test";
import {
  generateAptPackages,
  generateAptRelease,
  generateAptSourcesList,
  generateAptSourcesDeb822,
  generateRpmPrimaryXml,
  generateRpmRepomdXml,
  generateRpmRepoConfig,
  resolvePackageDownloadUrl,
  LINUX_REPO_APPS,
} from "../src/lib/linux-repos";
import { gzipSync } from "node:zlib";

test.describe("Linux Package Repositories (APT & RPM)", () => {
  test("resolves release download URLs for deb and rpm packages", () => {
    const debUrl = resolvePackageDownloadUrl("pdfcraft-0.16.1-linux-x86_64.deb");
    expect(debUrl).toBe("https://github.com/storytold/pdfcraft/releases/download/v0.16.1/pdfcraft-0.16.1-linux-x86_64.deb");

    const rpmUrl = resolvePackageDownloadUrl("pdfcraft-0.16.1-linux-x86_64.rpm");
    expect(rpmUrl).toBe("https://github.com/storytold/pdfcraft/releases/download/v0.16.1/pdfcraft-0.16.1-linux-x86_64.rpm");

    const launcherUrl = resolvePackageDownloadUrl("artcraft-launcher-0.11.1-linux-riscv64.deb");
    expect(launcherUrl).toBe("https://github.com/storytold/craft-launcher/releases/download/v0.11.1/artcraft-launcher-0.11.1-linux-riscv64.deb");

    const invalid = resolvePackageDownloadUrl("some-random-file.tar.gz");
    expect(invalid).toBeNull();
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

  test("generates valid RPM primary metadata XML", () => {
    const primaryXml = generateRpmPrimaryXml();
    expect(primaryXml).toContain('<metadata xmlns="http://linux.duke.edu/metadata/common"');
    expect(primaryXml).toContain("<name>pdfcraft</name>");
    expect(primaryXml).toContain("<arch>x86_64</arch>");
    expect(primaryXml).toContain('<location href="packages/pdfcraft-0.16.1-linux-x86_64.rpm"/>');
    expect(primaryXml).toContain("<arch>aarch64</arch>");
    expect(primaryXml).toContain("<arch>riscv64</arch>");
    expect(primaryXml).toContain("<name>artcraft-launcher</name>");

    const x86Xml = generateRpmPrimaryXml(1730000000, "x86_64");
    expect(x86Xml).toContain("<arch>x86_64</arch>");
    expect(x86Xml).not.toContain("<arch>aarch64</arch>");
    expect(x86Xml).not.toContain("<arch>riscv64</arch>");

    const riscvXml = generateRpmPrimaryXml(1730000000, "riscv64");
    expect(riscvXml).toContain("<arch>riscv64</arch>");
    expect(riscvXml).not.toContain("<arch>x86_64</arch>");
  });

  test("generates valid repomd.xml pointing to primary.xml.gz", () => {
    const primaryXml = generateRpmPrimaryXml();
    const primaryBuffer = Buffer.from(primaryXml, "utf-8");
    const gzBuffer = gzipSync(primaryBuffer);

    const repomd = generateRpmRepomdXml(primaryBuffer, gzBuffer);
    expect(repomd).toContain('<repomd xmlns="http://linux.duke.edu/metadata/repo">');
    expect(repomd).toContain('<data type="primary">');
    expect(repomd).toContain('<location href="repodata/primary.xml.gz"/>');
  });

  test("generates RPM repo config with gpgcheck=0", () => {
    const repo = generateRpmRepoConfig("https://getartcraft.com");
    expect(repo).toContain("[artcraft]");
    expect(repo).toContain("baseurl=https://getartcraft.com/rpm");
    expect(repo).toContain("gpgcheck=0");
  });
});
