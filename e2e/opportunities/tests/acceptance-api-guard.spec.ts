import { expect, test } from "@playwright/test";

/**
 * Positive control: fixture server allows reset only on the e2e workspace (Rust forbids others).
 */
test.describe("Acceptance API on fixture server", () => {
  test.describe.configure({ mode: "serial" });

  test("reset succeeds on e2e workspace", async ({ request }) => {
    const res = await request.post("/api/_acceptance/reset-opportunities");
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.ok).toBe(true);
  });
});
