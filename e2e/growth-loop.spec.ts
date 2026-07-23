import { expect, test } from "@playwright/test"

test("creates a controlled growth action before approval", async ({ page }) => {
  await page.goto("/growth")
  await page.getByLabel("Action title").fill("Join one founder conversation")
  await page.getByLabel("Why this belongs in today’s queue").fill("The author matches the active ICP.")
  await page.getByLabel("Exact action or content").fill("A concrete and useful comment for this founder.")
  await page.getByLabel("Experiment hypothesis").fill("Specific comments create qualified replies.")
  await page.getByRole("button", { name: "Add to controlled queue" }).click()

  await expect(page.getByRole("heading", { name: "Join one founder conversation" })).toBeVisible()
  await expect(page.getByRole("button", { name: "Approve exact revision" })).toBeEnabled()
})
