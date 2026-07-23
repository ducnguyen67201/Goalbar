import { expect, test } from "@playwright/test"

test("completes local preview onboarding", async ({ page }) => {
  await page.goto("/onboarding")
  await page.getByLabel("Your name").fill("Duc")
  await page.getByLabel("Product or project").fill("Acme")
  await page.getByLabel("What do you offer?").fill("A local sustainable growth system")
  await page.getByLabel("What have you earned the right to talk about?").fill("Building local-first products")
  await page.getByRole("button", { name: /save founder baseline/i }).click()
  await expect(page.getByRole("heading", { name: /build a growth loop/i })).toBeVisible()
})
