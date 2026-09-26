import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 45_000,
  expect: { timeout: 15_000 },
  workers: 1,
  use: {
    // Use installed Google Chrome; no Playwright browser download is needed.
    channel: "chrome",
    baseURL: "http://localhost:4202",
    viewport: { width: 1440, height: 960 },
    contextOptions: { reducedMotion: "reduce" },
    trace: "retain-on-failure",
    launchOptions: { args: ["--enable-unsafe-swiftshader"] },
  },
  webServer: [
    {
      command: "node tests/fixtures/api-server.mjs",
      url: "http://127.0.0.1:4203/health",
      reuseExistingServer: false,
    },
    {
      command: "npm run dev -- --hostname localhost --port 4202",
      url: "http://localhost:4202/media/m_fixture_image",
      // Server-rendered social metadata also needs fixtures. Never reuse a
      // developer's server, which could still be configured for production.
      env: { NEXT_PUBLIC_API_HOST: "http://127.0.0.1:4203" },
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
});
