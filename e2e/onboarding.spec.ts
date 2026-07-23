import { expect, test } from "@playwright/test"

test("completes local preview onboarding", async ({ page }) => {
  await page.goto("/onboarding")
  await page.getByLabel("Your name").fill("Duc")
  await page.getByLabel("Product or company").fill("Acme")
  await page.getByLabel("Paste your landing page").fill("https://acme.example")
  await page.getByLabel("Describe your ICP").fill("Technical solo founders building local-first products")
  await page.getByRole("button", { name: /create my starting profile/i }).click()
  await expect(page.getByRole("heading", { name: /build a growth loop/i })).toBeVisible()
})
