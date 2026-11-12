import { expect, test } from "@playwright/test";

const FIXTURE_URL = "https://example.com/jobdesk-e2e/job/acceptance-1";
const USER_TITLE = "User Edited Title";

test.describe("Opportunities prior-review acceptance", () => {
  test.describe.configure({ mode: "serial" });

  test.beforeEach(async ({ request }) => {
    const reset = await request.post("/api/_acceptance/reset-opportunities");
    expect(reset.ok()).toBeTruthy();
  });

  test("complete workflow preserves review history and visibility", async ({
    page,
    request,
  }) => {
    await page.goto("/");
    await page.getByTestId("tab-opportunities").click();
    await expect(page.locator("#panel-opportunities")).toBeVisible();

    await page.getByTestId("opp-url").fill(FIXTURE_URL);
    const ingestResponse = page.waitForResponse(
      (r) =>
        r.url().includes("/api/opportunities/ingest") &&
        r.request().method() === "POST",
    );
    await page.getByTestId("opp-fetch-save").click();
    const ingestRes = await ingestResponse;
    expect(ingestRes.ok()).toBeTruthy();
    const ingestBody = await ingestRes.json();
    expect(ingestBody.posting?.prior_reviewed).toBeFalsy();
    await expect(page.getByTestId("opp-list")).toContainText("Role V1", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("opp-detail")).toBeVisible();
    await expect(page.getByTestId("opp-prior-review-banner")).toBeHidden();

    await page.getByTestId("opp-field-title").fill(USER_TITLE);
    const saveResponse = page.waitForResponse(
      (r) =>
        r.url().includes("/interpretation") && r.request().method() === "PATCH",
    );
    await page.getByTestId("opp-save-edits").click();
    const saveRes = await saveResponse;
    expect(saveRes.ok()).toBeTruthy();

    const markReviewResponse = page.waitForResponse(
      (r) =>
        r.url().includes("/interpretation") && r.request().method() === "PATCH",
    );
    await page.getByTestId("opp-mark-reviewed").click();
    const reviewedRes = await markReviewResponse;
    expect(reviewedRes.ok()).toBeTruthy();
    const reviewedBody = await reviewedRes.json();
    const reviewedInterpretationId: string = reviewedBody.current.id;
    const reviewedAt: string = reviewedBody.current.reviewed_at;
    expect(reviewedAt).toBeTruthy();

    const reviewLine = page.getByTestId("opp-review-line");
    await expect(reviewLine).toBeVisible();
    await expect(reviewLine).toContainText("Reviewed at");
    await expect(reviewLine).toContainText(reviewedAt.slice(0, 19));

    await page.reload();
    await page.getByTestId("tab-opportunities").click();
    await expect(page.getByTestId("opp-list")).toContainText(USER_TITLE);
    await page
      .getByTestId("opp-list")
      .getByRole("button", { name: new RegExp(USER_TITLE) })
      .click();
    await expect(page.getByTestId("opp-detail")).toBeVisible();
    await expect(page.getByTestId("opp-field-title")).toHaveValue(USER_TITLE);
    await expect(reviewLine).toContainText("Reviewed at");

    await page.getByTestId("opp-url").fill(FIXTURE_URL);
    const refetchResponse = page.waitForResponse(
      (r) =>
        r.url().includes("/api/opportunities/ingest") &&
        r.request().method() === "POST",
    );
    await page.getByTestId("opp-fetch-save").click();
    const refetchRes = await refetchResponse;
    expect(refetchRes.ok()).toBeTruthy();
    await expect(page.getByTestId("opp-field-title")).toHaveValue("Role V2", {
      timeout: 30_000,
    });
    await expect(reviewLine).toContainText("Not reviewed");
    await expect(reviewLine).not.toContainText("Reviewed at");

    const banner = page.getByTestId("opp-prior-review-banner");
    await expect(banner).toBeVisible();
    await expect(banner).toContainText("Earlier reviewed interpretation");
    await expect(banner).toContainText(reviewedAt.slice(0, 19));
    await expect(banner).toHaveAttribute(
      "data-prior-interpretation-id",
      reviewedInterpretationId,
    );

    await expect(page.getByTestId("opp-list")).toContainText("prior review");
    await expect(page.getByTestId("opp-list")).toContainText(
      reviewedAt.slice(0, 19),
    );

    const appsRes = await request.get("/api/applications");
    expect(appsRes.ok()).toBeTruthy();
    const apps = await appsRes.json();
    expect(apps).toEqual([]);
  });
});
