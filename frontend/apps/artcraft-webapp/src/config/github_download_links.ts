//const WINDOWS_VERSION = "0.7.0"; // 2026-02-23 Seedance
//const WINDOWS_VERSION = "0.11.0"; // 2026-03-02 Credits accounting
//const WINDOWS_VERSION = "0.14.0"; // 2026-03-10 WorldLabs native, Angles
//const WINDOWS_VERSION = "0.20.0"; // 2026-03-24 Reprompt, performance
//const WINDOWS_VERSION = "0.28.0"; // 2026-04-03 characters + prompt box
//const WINDOWS_VERSION = "0.41.0";
const ARTCRAFT_STUDIO_VERSION = "0.41.1";
const WINDOWS_VERSION = ARTCRAFT_STUDIO_VERSION;
const MAC_VERSION = ARTCRAFT_STUDIO_VERSION;
const LINUX_VERSION = ARTCRAFT_STUDIO_VERSION;

export const DOWNLOAD_LINKS = {
  WINDOWS: `https://github.com/storytold/artcraft/releases/download/artcraft-v${WINDOWS_VERSION}/ArtCraft_${WINDOWS_VERSION}_x64-setup.exe`,
  MACOS: `https://github.com/storytold/artcraft/releases/download/artcraft-v${MAC_VERSION}/ArtCraft_${MAC_VERSION}_universal.dmg`,
  LINUX: `https://github.com/storytold/artcraft/releases/download/artcraft-v${LINUX_VERSION}/artcraft-${LINUX_VERSION}-linux-x86_64.flatpak`,
} as const;
