import { expect, test } from "@playwright/test"

test("browser workbench gives persistent Codex chat a bounded Browser Use tool", async ({ page }) => {
  await page.goto("/browser")
  await expect(page.getByRole("heading", { name: "Chat with the browser beside you." })).toBeVisible()
  await expect(page.getByRole("region", { name: "Founder chat" })).toBeVisible()
  await expect(page.getByRole("heading", { name: "Where do you want to research?" })).toBeVisible()
  await expect(page.getByRole("button", { name: /browser use chat callable/i })).toBeVisible()
  await expect(page.getByRole("region", { name: "Local agent terminals" })).toHaveCount(0)
  await expect(
    page.getByRole("button", { name: /^(publish|publish approved text|send reply)$/i }),
  ).toHaveCount(0)

  await page.getByRole("button", { name: /open x/i }).click()
  await page.getByRole("textbox", { name: "Chat message" }).fill("Find me good 5 posts for ICP signals")
  await page.getByRole("button", { name: "Send message" }).click()
  await expect(page.getByRole("region", { name: "Codex Browser Use activity" })).toBeVisible()
  await expect(page.getByText("Browser Use complete", { exact: true })).toBeVisible()
  await expect(page.getByText("Called directly by the persistent Codex chat")).toBeVisible()
  await expect(page.getByRole("button", { name: "Run approved research" })).toHaveCount(0)

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
