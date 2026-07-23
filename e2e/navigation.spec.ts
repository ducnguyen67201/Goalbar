import { expect, test } from "@playwright/test"

test("main command-center routes are reachable", async ({ page }) => {
  await page.goto("/")
  await expect(page.getByText("Local only")).toBeVisible()
  await page.getByRole("link", { name: "Browser" }).click()
  await expect(page.getByRole("heading", { name: /research without switching/i })).toBeVisible()
  await page.getByRole("link", { name: "Settings" }).click()
  await expect(page.getByRole("heading", { name: /connections without credential custody/i })).toBeVisible()
  await page.getByRole("link", { name: "Growth", exact: true }).click()
  await expect(page.getByRole("heading", { name: /progress you can explain/i })).toBeVisible()
})
