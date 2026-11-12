import fs from "node:fs";
import path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const port = process.env.JOBDESK_E2E_PORT ?? "19547";
const baseURL = `http://127.0.0.1:${port}`;

function nixBrowserExe(prefix: string, suffix: string): string | undefined {
  const root =
    process.env.NIX_PLAYWRIGHT_BROWSERS ?? process.env.PLAYWRIGHT_BROWSERS_PATH;
  if (!root || !fs.existsSync(root)) return undefined;
  const dir = fs
    .readdirSync(root)
    .find((d) => d.startsWith(`${prefix}-`));
  if (!dir) return undefined;
  const exe = path.join(root, dir, suffix);
  return fs.existsSync(exe) ? exe : undefined;
}

const envFirefox = process.env.JOBDESK_E2E_FIREFOX;
const envChromium = process.env.JOBDESK_E2E_CHROMIUM;
const nixFirefox = envFirefox ?? nixBrowserExe("firefox", "firefox/firefox");
const nixChromium =
  envChromium ??
  nixBrowserExe(
    "chromium_headless_shell",
    "chrome-headless-shell-linux64/chrome-headless-shell",
  );

const projects = [];
if (nixFirefox && fs.existsSync(nixFirefox)) {
  projects.push({
    name: "firefox",
    use: {
      ...devices["Desktop Firefox"],
      launchOptions: { executablePath: nixFirefox },
    },
  });
}
if (nixChromium && fs.existsSync(nixChromium)) {
  projects.push({
    name: "chromium",
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: { executablePath: nixChromium },
    },
  });
}
if (projects.length === 0) {
  projects.push(
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
  );
}

// One worker + non-parallel: Firefox/Chromium never run against the same DB concurrently.
// `beforeEach` reset + fixture sequence rewind isolate sequential projects on one server.
export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [["list"], ["html", { open: "never", outputFolder: "../../target/e2e-opportunities-report" }]],
  outputDir: "../../target/e2e-opportunities-artifacts",
  use: {
    baseURL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects,
  webServer: {
    command: "../../scripts/jobdesk-e2e-server.sh",
    url: `${baseURL}/api/health`,
    reuseExistingServer: false,
    timeout: 180_000,
    env: {
      ...process.env,
      JOBDESK_E2E_PORT: port,
    },
  },
});
