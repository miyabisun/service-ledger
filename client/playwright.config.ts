import { defineConfig } from "@playwright/test";

// E2E runs the real server binary on a throwaway database with fictional data.
// Build it first: `npm run build` here, then `cargo build` at the repository root.
export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  use: { baseURL: "http://127.0.0.1:5187", browserName: "chromium" },
  webServer: {
    command:
      "rm -rf ../target/e2e-data && DB_PATH=../target/e2e-data/ledger.db PORT=5187 LOG_LEVEL=warn ../target/debug/service-ledger",
    url: "http://127.0.0.1:5187/healthz",
    reuseExistingServer: false,
  },
});
