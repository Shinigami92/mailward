import { defineConfig, devices } from "@playwright/test";

// When E2E_BASE_URL is set (e.g. a running `pnpm dev` + API), test against it directly;
// otherwise build the SPA and serve it via `vite preview`.
const externalBaseUrl = process.env.E2E_BASE_URL ?? "";
const hasExternalBaseUrl = externalBaseUrl !== "";
const isCI = Boolean(process.env.CI);

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: isCI,
  retries: isCI ? 2 : 0,
  reporter: "list",
  use: {
    baseURL: hasExternalBaseUrl ? externalBaseUrl : "http://localhost:4173",
    trace: "on-first-retry",
  },
  webServer: hasExternalBaseUrl
    ? undefined
    : {
        command: "pnpm build && pnpm preview --port 4173 --strictPort",
        url: "http://localhost:4173",
        reuseExistingServer: !isCI,
        timeout: 120_000,
      },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
