import { beforeEach, describe, expect, it, vi } from "vitest"
import { z } from "zod"

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }))
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }))

import { invokeValidated } from "./tauri"

describe("typed Tauri boundary", () => {
  beforeEach(() => invokeMock.mockReset())

  it("validates input and output", async () => {
    invokeMock.mockResolvedValue({ value: 2 })
    await expect(
      invokeValidated(
        "double",
        { input: { value: 1 } },
        z.object({ value: z.number() }),
        z.object({ value: z.number() }),
      ),
    ).resolves.toEqual({ value: 2 })
    expect(invokeMock).toHaveBeenCalledWith("double", { input: { value: 1 } })
  })

  it("rejects malformed output", async () => {
    invokeMock.mockResolvedValue({ value: "wrong" })
    await expect(
      invokeValidated(
        "double",
        { input: { value: 1 } },
        z.object({ value: z.number() }),
        z.object({ value: z.number() }),
      ),
    ).rejects.toThrow()
  })
})
