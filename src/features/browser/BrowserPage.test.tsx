import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it } from "vitest"

import { BrowserPage } from "./BrowserPage"

function renderBrowser() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <BrowserPage />
    </QueryClientProvider>,
  )
}

describe("BrowserPage preview mode", () => {
  it("opens on a blank platform chooser", () => {
    renderBrowser()
    expect(screen.getByRole("heading", { name: "Research without switching." })).toBeInTheDocument()
    expect(screen.getByRole("heading", { name: "Where do you want to research?" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /open x/i })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /open linkedin/i })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /open reddit/i })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Preview visible" })).toBeDisabled()
    expect(screen.getByRole("button", { name: "Check policy and start" })).toBeDisabled()
    expect(screen.queryByText(/you still review and click publish or send/i)).not.toBeInTheDocument()
  })

  it("requires explicit confirmation of collection bounds", async () => {
    const user = userEvent.setup()
    renderBrowser()
    await user.click(screen.getByRole("button", { name: /open x/i }))
    await user.click(screen.getByRole("checkbox", { name: /confirm this objective/i }))
    expect(screen.getByRole("button", { name: "Check policy and start" })).toBeEnabled()
  })

  it("keeps the address visible and normalizes an HTTPS navigation", async () => {
    const user = userEvent.setup()
    renderBrowser()
    const address = screen.getByRole("textbox", { name: "Browser address" })
    await user.clear(address)
    await user.type(address, "reddit.com/r/startups{Enter}")
    await waitFor(() =>
      expect(screen.getByRole("textbox", { name: "Browser address" })).toHaveValue(
        "https://reddit.com/r/startups",
      ),
    )
  })

  it("opens the platform chooser when a new tab is requested", async () => {
    const user = userEvent.setup()
    renderBrowser()
    await user.click(screen.getByRole("button", { name: /open linkedin/i }))
    expect(screen.queryByRole("heading", { name: "Where do you want to research?" })).not.toBeInTheDocument()

    await user.click(screen.getByRole("button", { name: "New browser tab" }))

    expect(screen.getByRole("heading", { name: "Where do you want to research?" })).toBeInTheDocument()
  })
})
