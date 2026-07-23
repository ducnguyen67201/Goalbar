import { describe, expect, it } from "vitest"

import { bootstrapFixture } from "@/test/fixtures"

import { bootstrapSchema } from "./bootstrap"
import { founderInputSchema } from "./founder"
import { beginOAuthInputSchema } from "./platform"

describe("boundary schemas", () => {
  it("accepts the versioned bootstrap contract", () => {
    expect(bootstrapSchema.parse(bootstrapFixture).schemaVersion).toBe(1)
  })

  it("rejects an empty founder profile", () => {
    expect(
      founderInputSchema.safeParse({
        name: "",
        productName: "",
        offer: "",
        expertise: "",
        goals: [],
        boundaries: [],
      }).success,
    ).toBe(false)
  })

  it("requires local OAuth account identifiers", () => {
    expect(
      beginOAuthInputSchema.safeParse({
        platform: "x",
        clientId: "",
        remoteAccountId: "",
        displayName: "",
        scopes: [],
      }).success,
    ).toBe(false)
  })
})
