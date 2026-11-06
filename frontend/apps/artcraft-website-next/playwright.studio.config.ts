import { defineConfig } from "@playwright/test";
export default defineConfig({ testDir: "./tests", testMatch: "studio-linux.spec.ts", workers: 1 });
