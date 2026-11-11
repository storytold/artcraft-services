import { defineConfig } from "@playwright/test";
export default defineConfig({ testDir: "./tests", testMatch: ["studio-linux.spec.ts", "flatpak-oci.spec.ts"], workers: 1 });
