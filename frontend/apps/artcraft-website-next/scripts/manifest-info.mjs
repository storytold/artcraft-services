import { LINUX_REPO_APPS } from "../src/lib/linux-repos.ts";

const mode = process.argv[2];
const targetArch = process.argv[3];

switch (mode) {
  case "apt-packages": {
    // Package identifiers for apt-get download: <name>:<arch>
    const pkgs = LINUX_REPO_APPS.flatMap((app) =>
      app.debAssets.map((asset) => `${app.name}:${asset.arch}`)
    );
    console.log(pkgs.join(" "));
    break;
  }

  case "apt-install-packages":
  case "dnf-install-packages": {
    // Unique package names to install on native system
    const names = Array.from(new Set(LINUX_REPO_APPS.map((app) => app.name)));
    console.log(names.join(" "));
    break;
  }

  case "apt-checksums": {
    // sha256 checksum file content for sha256sum -c
    // apt-get download stores files as <name>_<version>-1_<arch>.deb
    const lines = LINUX_REPO_APPS.flatMap((app) =>
      app.debAssets.map((asset) => {
        const debFilename = `${app.name}_${app.version}-1_${asset.arch}.deb`;
        return `${asset.sha256}  ${debFilename}`;
      })
    );
    console.log(lines.join("\n"));
    break;
  }

  case "dnf-packages": {
    // Package identifiers for dnf download: <name>.<arch>
    const pkgs = LINUX_REPO_APPS.flatMap((app) =>
      app.rpmAssets.map((asset) => `${app.name}.${asset.arch}`)
    );
    console.log(pkgs.join(" "));
    break;
  }

  case "rpm-checksums": {
    // sha256 checksum file content for sha256sum -c
    // dnf and zypper download store files as <asset.filename>
    const lines = LINUX_REPO_APPS.flatMap((app) =>
      app.rpmAssets.map((asset) => `${asset.sha256}  ${asset.filename}`)
    );
    console.log(lines.join("\n"));
    break;
  }

  case "zypper-packages": {
    if (!targetArch) {
      console.error("Error: Arch argument required for zypper-packages");
      process.exit(1);
    }
    const pkgs = LINUX_REPO_APPS.filter((app) =>
      app.rpmAssets.some((a) => a.arch === targetArch)
    ).map((app) => `artcraft-${targetArch}:${app.name}`);
    console.log(pkgs.join(" "));
    break;
  }

  case "zypper-install-packages": {
    // Package names to install using zypper
    const names = Array.from(new Set(LINUX_REPO_APPS.map((app) => `artcraft:${app.name}`)));
    console.log(names.join(" "));
    break;
  }

  case "flatpak-apps": {
    const apps = Array.from(new Set(LINUX_REPO_APPS.map((app) => app.name))).sort((a, b) => a.localeCompare(b));
    console.log(apps.join(" "));
    break;
  }

  default: {
    console.error(`Unknown mode: ${mode}`);
    console.error("Supported modes: apt-packages, apt-install-packages, apt-checksums, dnf-packages, dnf-install-packages, rpm-checksums, zypper-packages <arch>, zypper-install-packages, flatpak-apps");
    process.exit(1);
  }
}
