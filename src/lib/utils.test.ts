import { describe, expect, it } from "vitest"

import { relativeDate } from "./dates"
import { titleCase } from "./utils"

describe("presentation helpers", () => {
  it("formats enum labels", () => expect(titleCase("approval_pending")).toBe("Approval Pending"))

  it("formats near dates relatively", () => {
    expect(relativeDate("2026-07-22T01:00:00Z", Date.parse("2026-07-22T00:00:00Z"))).toBe("in 1 hour")
  })
})
