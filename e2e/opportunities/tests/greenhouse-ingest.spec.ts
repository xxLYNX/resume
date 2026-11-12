import { test, expect } from "@playwright/test";

const GH_URL =
  "https://job-boards.greenhouse.io/flexport/jobs/8009198";

test.describe.configure({ mode: "serial" });

test.describe("Greenhouse adapter (fixture API)", () => {
  test.beforeEach(async ({ request }) => {
    const reset = await request.post("/api/_acceptance/reset-opportunities");
    expect(reset.ok()).toBeTruthy();
  });

  test("ingest shows recognized fields and JSON source", async ({ page }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(GH_URL);
    const ingestResponse = page.waitForResponse(
      (r) =>
        r.url().includes("/api/opportunities/ingest") &&
        r.request().method() === "POST",
    );
    await page.getByTestId("opp-fetch-save").click();
    const ingestRes = await ingestResponse;
    expect(ingestRes.status()).toBe(201);
    await expect(page.getByTestId("opp-list")).toContainText(
      "Account Executive",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("opp-detail")).toBeVisible();
    await expect(page.locator("#opp-extraction-line")).toContainText(
      /recognized/i,
    );
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      "Account Executive",
    );
    await expect(page.locator("#opp-source-preview")).toContainText("8009198", {
      timeout: 15_000,
    });
  });

  test("ingest does not create applications", async ({ page, request }) => {
    const before = await request.get("/api/applications");
    const beforeBody = await before.json();
    const countBefore = Array.isArray(beforeBody) ? beforeBody.length : 0;
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(GH_URL);
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.getByTestId("opp-list")).toContainText("Account Executive", {
      timeout: 30_000,
    });
    const after = await request.get("/api/applications");
    const afterBody = await after.json();
    const countAfter = Array.isArray(afterBody) ? afterBody.length : 0;
    expect(countAfter).toBe(countBefore);
  });

  test("re-fetch mismatch keeps last usable content", async ({ page }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(GH_URL);
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.getByTestId("opp-list")).toContainText(
      "Account Executive",
      { timeout: 30_000 },
    );
    await page.getByTestId("opp-url").fill(GH_URL);
    const refetch = page.waitForResponse((r) =>
      r.url().includes("/api/opportunities/ingest"),
    );
    await page.getByTestId("opp-fetch-save").click();
    await refetch;
    await expect(page.locator("#opp-extraction-line")).toContainText(
      /last usable/i,
    );
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      "Account Executive",
    );
  });

  test("review then transport-failed re-fetch keeps reviewed data", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(GH_URL);
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      "Account Executive",
      { timeout: 30_000 },
    );
    await page.getByTestId("opp-mark-reviewed").click();
    await expect(page.locator("#opp-review-line")).toContainText(/Reviewed at/i);
    await page.getByTestId("opp-url").fill(GH_URL);
    await page.getByTestId("opp-fetch-save").click();
    await page.getByTestId("opp-fetch-save").click();
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.locator("#opp-extraction-line")).toContainText(
      /Latest fetch failed/i,
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      "Account Executive",
    );
  });

  test("stale tab edit surfaces conflict after re-fetch", async ({
    browser,
  }) => {
    const fixtureUrl = "https://example.com/jobdesk-e2e/job/acceptance-1";
    const ctxA = await browser.newContext();
    const ctxB = await browser.newContext();
    const pageA = await ctxA.newPage();
    const pageB = await ctxB.newPage();
    for (const p of [pageA, pageB]) {
      await p.goto("/");
      await p.getByTestId("tab-opportunities").click();
    }
    await pageA.getByTestId("opp-url").fill(fixtureUrl);
    await pageA.getByTestId("opp-fetch-save").click();
    await expect(pageA.getByTestId("opp-list")).toContainText("Role V1", {
      timeout: 30_000,
    });
    await pageB.goto("/");
    await pageB.getByTestId("tab-opportunities").click();
    await expect(pageB.getByTestId("opp-list")).toContainText("Role V1", {
      timeout: 30_000,
    });
    await pageB.getByTestId("opp-list").getByRole("button").first().click();
    await expect(pageB.getByTestId("opp-field-title")).toHaveValue("Role V1", {
      timeout: 15_000,
    });
    await pageB.getByTestId("opp-field-title").fill("Stale tab title");
    await pageA.getByTestId("opp-url").fill(fixtureUrl);
    await pageA.getByTestId("opp-fetch-save").click();
    await expect(pageA.getByTestId("opp-field-title")).toHaveValue("Role V2", {
      timeout: 30_000,
    });
    const staleSave = pageB.waitForResponse(
      (r) =>
        r.url().includes("/interpretation") && r.request().method() === "PATCH",
    );
    await pageB.getByTestId("opp-save-edits").click();
    const saveRes = await staleSave;
    expect(saveRes.status()).toBe(409);
    await ctxA.close();
    await ctxB.close();
  });
});
