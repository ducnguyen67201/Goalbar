import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it } from "vitest"

import { GrowthPage } from "./GrowthPage"

describe("GrowthPage", () => {
  it("adds a fully specified action to the controlled queue", async () => {
    const user = userEvent.setup()
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })
    render(
      <QueryClientProvider client={client}>
        <GrowthPage />
      </QueryClientProvider>,
    )

    expect(await screen.findByText("Nothing acts silently.")).toBeInTheDocument()
    await user.type(screen.getByLabelText("Action title"), "Join one founder conversation")
    await user.type(
      screen.getByLabelText("Why this belongs in today’s queue"),
      "The author matches the active ICP.",
    )
    await user.type(
      screen.getByLabelText("Exact action or content"),
      "A concrete and useful comment for this founder.",
    )
    await user.type(
      screen.getByLabelText("Experiment hypothesis"),
      "Specific comments create qualified replies.",
    )
    await user.click(screen.getByRole("button", { name: "Add to controlled queue" }))

    expect(await screen.findByRole("heading", { name: "Join one founder conversation" })).toBeVisible()
    expect(screen.getByRole("button", { name: "Approve exact revision" })).toBeEnabled()
    expect(screen.getByText("Revision 1")).toBeInTheDocument()
  })
})
