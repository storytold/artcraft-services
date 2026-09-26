// Desktop installer links, ported from the Vite site's
// config/github_download_links.ts. Keep previous releases in the history
// when adding new downloads, newest first.
const DOWNLOAD_HISTORY = {
  "0.41.0": githubDownloadLinks("0.41.0"),
  "0.37.0": githubDownloadLinks("0.37.0"),
  "0.32.0": githubDownloadLinks("0.32.0"),
  "0.28.0": githubDownloadLinks("0.28.0"),
  "0.23.0": githubDownloadLinks("0.23.0"), // 2026-04-03 Characters + prompt box
  "0.20.0": githubDownloadLinks("0.20.0"), // 2026-03-24 Reprompt, performance
  "0.14.0": githubDownloadLinks("0.14.0"), // 2026-03-10 WorldLabs native, Angles
  "0.11.0": githubDownloadLinks("0.11.0"), // 2026-03-02 Credits accounting
  "0.7.0": githubDownloadLinks("0.7.0"), // 2026-02-23 Seedance
} as const;

// To roll back either platform, select a version from DOWNLOAD_HISTORY.
const WINDOWS_VERSION: keyof typeof DOWNLOAD_HISTORY = "0.41.0";
const MAC_VERSION: keyof typeof DOWNLOAD_HISTORY = "0.41.0";

// Set false to temporarily hide desktop downloads throughout the website.
export const DOWNLOADS_ENABLED = true;

export const DOWNLOAD_LINKS = {
  WINDOWS: DOWNLOAD_HISTORY[WINDOWS_VERSION].WINDOWS,
  MACOS: DOWNLOAD_HISTORY[MAC_VERSION].MACOS,
} as const;

export const DOWNLOAD_VERSIONS = {
  WINDOWS: WINDOWS_VERSION,
  MACOS: MAC_VERSION,
} as const;

function githubDownloadLinks(version: string) {
  const baseUrl = `https://github.com/storytold/artcraft/releases/download/artcraft-v${version}`;
  return {
    WINDOWS: `${baseUrl}/ArtCraft_${version}_x64-setup.exe`,
    MACOS: `${baseUrl}/ArtCraft_${version}_universal.dmg`,
  } as const;
}
