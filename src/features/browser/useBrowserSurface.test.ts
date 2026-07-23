import { describe, expect, it } from "vitest"

import type { BrowserTab } from "@/schemas/browser"

import { deduplicateBrowserTabs, upsertBrowserTab } from "./useBrowserSurface"

function browserTab(id: string, active = false): BrowserTab {
  return {
    id,
    webviewLabel: `browser-${id}`,
    currentUrl: "https://x.com/home",
    title: "Home / X",
    loadState: "loaded",
    platform: "x",
    active,
    createdAt: "2026-07-23T04:00:00+00:00",
  }
}

describe("browser tab reconciliation", () => {
  it("collapses repeated reports for the same native tab", () => {
    const tab = browserTab("4c227eb4-fbf6-4a32-9a41-c31a62a596f5", true)

    expect(deduplicateBrowserTabs([tab, { ...tab, title: "Updated title" }])).toEqual([
      { ...tab, title: "Updated title" },
    ])
  })

  it("upserts an event by ID and keeps only the reported tab active", () => {
    const first = browserTab("4c227eb4-fbf6-4a32-9a41-c31a62a596f5", true)
    const second = browserTab("7dd055b7-f370-4105-b9bc-84f0db3fe38e")
    const updatedSecond = { ...second, title: "LinkedIn", active: true }

    expect(upsertBrowserTab([first, second, second], updatedSecond)).toEqual([
      { ...first, active: false },
      updatedSecond,
    ])
  })
})
