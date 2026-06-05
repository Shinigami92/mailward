import { expect, test } from "@playwright/test";

test("renders the app shell, nav and run view", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "mailward", exact: true })).toBeVisible();
  for (const label of ["Run", "Accounts", "Config", "Rules"]) {
    await expect(page.getByRole("link", { name: label })).toBeVisible();
  }
  await expect(page.getByRole("button", { name: "Dry run" })).toBeVisible();
  await page.screenshot({ path: "test-results/run-view.png", fullPage: true });
});

test("navigates to the accounts view", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Accounts" }).click();
  await expect(page).toHaveURL(/\/accounts$/);
  await page.screenshot({ path: "test-results/accounts-view.png", fullPage: true });
});
