import { expect, test } from "@playwright/test"

test("browser workbench exposes bounded local controls in preview mode", async ({ page }) => {
  await page.goto("/browser")
  await expect(page.getByRole("heading", { name: "Research without switching." })).toBeVisible()
  await expect(page.getByRole("heading", { name: "Integrated browser preview" })).toBeVisible()
  await expect(page.getByRole("button", { name: "Preview visible" })).toBeVisible()
  await expect(page.getByRole("button", { name: "Check policy and start" })).toBeVisible()
  await expect(page.getByRole("button", { name: /publish|send/i })).toHaveCount(0)

  const address = page.getByRole("textbox", { name: "Browser address" })
  await address.fill("reddit.com/r/startups")
  await address.press("Enter")
  await expect(address).toHaveValue("https://reddit.com/r/startups")
})

test("settings presents browser first and labels APIs optional", async ({ page }) => {
  await page.goto("/settings")
  await expect(page.getByRole("heading", { name: "Integrated browser" })).toBeVisible()
  await expect(page.getByRole("heading", { name: "Official API connections" })).toBeVisible()
  await expect(page.getByPlaceholder("Type CLEAR BROWSER DATA")).toBeVisible()
})
