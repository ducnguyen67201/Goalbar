import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it } from "vitest"

import { HistoryImportPanel } from "./HistoryImportPanel"

describe("HistoryImportPanel", () => {
  it("requires preview before committing the synthetic archive", async () => {
    const user = userEvent.setup()
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <HistoryImportPanel />
      </QueryClientProvider>,
    )

    await user.click(screen.getByRole("button", { name: /choose x, linkedin, or reddit archive/i }))
    expect(await screen.findByText("founder-archive.zip")).toBeInTheDocument()
    expect(screen.queryByRole("button", { name: /import normalized history/i })).not.toBeInTheDocument()

    await user.click(screen.getByRole("button", { name: "Preview" }))
    expect(await screen.findByText("240 records found")).toBeInTheDocument()
    await user.click(screen.getByRole("button", { name: "Import normalized history" }))
    expect(await screen.findByText(/imported 240; skipped 0/i)).toBeInTheDocument()
  })
})
