import { test, expect } from "@playwright/test";

const WD_INGEST_URL =
  "https://crowdstrike.wd5.myworkdayjobs.com/en-US/crowdstrikecareers/job/USA---Remote/Detection-Engineer--Falcon-Complete--Remote-_R30634";

const WD_CXS_URL =
  "https://crowdstrike.wd5.myworkdayjobs.com/wday/cxs/crowdstrike/crowdstrikecareers/job/USA---Remote/Detection-Engineer--Falcon-Complete--Remote-_R30634";

test.describe.configure({ mode: "serial" });

test.describe("Workday CXS adapter (fixture API)", () => {
  test.beforeEach(async ({ request }) => {
    const reset = await request.post("/api/_acceptance/reset-opportunities");
    expect(reset.ok()).toBeTruthy();
  });

  test("ingest shows recognized fields and JSON source", async ({ page }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(WD_INGEST_URL);
    const ingestResponse = page.waitForResponse(
      (r) =>
        r.url().includes("/api/opportunities/ingest") &&
        r.request().method() === "POST",
    );
    await page.getByTestId("opp-fetch-save").click();
    const ingestRes = await ingestResponse;
    expect(ingestRes.status()).toBe(201);
    await expect(page.getByTestId("opp-list")).toContainText(
      "Detection Engineer, Falcon Complete",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("opp-detail")).toBeVisible();
    await expect(page.locator("#opp-extraction-line")).toContainText(
      /recognized/i,
    );
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      /Detection Engineer, Falcon Complete \(Remote\)/,
    );
    await expect(page.locator("#opp-field-employer")).toHaveValue(
      "CrowdStrike, Inc.",
    );
    await expect(page.locator("#opp-field-location")).toHaveValue(/USA - Remote/);
    await expect(page.locator("#opp-source-preview")).toContainText("R30634", {
      timeout: 15_000,
    });
    await expect(page.locator("#opp-source-preview")).toContainText(
      "jobPostingInfo",
    );
  });

  test("fixture uses Workday CXS URL", async ({ request }) => {
    const reset = await request.post("/api/_acceptance/reset-opportunities");
    expect(reset.ok()).toBeTruthy();
    const ingest = await request.post("/api/opportunities/ingest", {
      data: { url: WD_INGEST_URL },
    });
    expect(ingest.status()).toBe(201);
    const body = await ingest.json();
    const postingId = body.posting?.posting?.id ?? body.posting?.id;
    expect(postingId).toBeTruthy();
    const detail = await request.get(`/api/opportunities/${postingId}`);
    expect(detail.ok()).toBeTruthy();
    const d = await detail.json();
    const snap = d.snapshots?.[0];
    expect(snap?.request_url).toBe(WD_CXS_URL);
    const source = await request.get(
      `/api/opportunities/snapshots/${snap.id}/source`,
    );
    expect(source.ok()).toBeTruthy();
    const text = await source.text();
    expect(text).toContain("jobPostingInfo");
    expect(text).toContain("R30634");
  });

  test("ingest does not create applications", async ({ page, request }) => {
    const before = await request.get("/api/applications");
    const beforeBody = await before.json();
    const countBefore = Array.isArray(beforeBody) ? beforeBody.length : 0;
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(WD_INGEST_URL);
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.getByTestId("opp-list")).toContainText(
      "Detection Engineer",
      { timeout: 30_000 },
    );
    const after = await request.get("/api/applications");
    const afterBody = await after.json();
    expect(afterBody.length).toBe(countBefore);
  });

  test("re-fetch mismatch keeps last usable content", async ({ page }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(WD_INGEST_URL);
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.getByTestId("opp-list")).toContainText(
      "Detection Engineer",
      { timeout: 30_000 },
    );
    await page.getByTestId("opp-url").fill(WD_INGEST_URL);
    const refetch = page.waitForResponse((r) =>
      r.url().includes("/api/opportunities/ingest"),
    );
    await page.getByTestId("opp-fetch-save").click();
    await refetch;
    await expect(page.locator("#opp-extraction-line")).toContainText(
      /last usable/i,
    );
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      /Detection Engineer, Falcon Complete \(Remote\)/,
    );
  });

  test("review then transport-failed re-fetch keeps reviewed data", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await page.getByTestId("opp-url").fill(WD_INGEST_URL);
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      /Detection Engineer/,
      { timeout: 30_000 },
    );
    await page.getByTestId("opp-mark-reviewed").click();
    await expect(page.locator("#opp-review-line")).toContainText(/Reviewed at/i);
    await page.getByTestId("opp-url").fill(WD_INGEST_URL);
    await page.getByTestId("opp-fetch-save").click();
    await page.getByTestId("opp-fetch-save").click();
    await page.getByTestId("opp-fetch-save").click();
    await expect(page.locator("#opp-extraction-line")).toContainText(
      /Latest fetch failed/i,
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("opp-field-title")).toHaveValue(
      /Detection Engineer, Falcon Complete \(Remote\)/,
    );
  });
});
