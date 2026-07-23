import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it } from "vitest"

import { responsiveBrowserPanelWidth } from "./browser-layout"
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
  it("keeps both workbench panes usable as the window resizes", () => {
    expect(responsiveBrowserPanelWidth(480, 892)).toBe(464)
    expect(responsiveBrowserPanelWidth(340, 1400)).toBe(340)
    expect(responsiveBrowserPanelWidth(100, 1400)).toBe(280)
  })

  it("opens on a blank platform chooser", () => {
    renderBrowser()
    expect(screen.getByRole("heading", { name: "Chat with the browser beside you." })).toBeInTheDocument()
    expect(screen.getByRole("region", { name: "Founder chat" })).toBeInTheDocument()
    expect(screen.getByRole("heading", { name: "Where do you want to research?" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /open x/i })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /open linkedin/i })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /open reddit/i })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /research chat callable/i })).toBeInTheDocument()
    expect(screen.queryByRole("region", { name: "Local agent terminals" })).not.toBeInTheDocument()
    expect(screen.queryByText("Research add-on requested")).not.toBeInTheDocument()
  })

  it("lets chat request research but requires explicit approval of its bounds", async () => {
    const user = userEvent.setup()
    renderBrowser()
    await user.click(screen.getByRole("button", { name: /open x/i }))
    await user.type(
      screen.getByRole("textbox", { name: "Chat message" }),
      "Research this feed for ICP pain signals",
    )
    await user.click(screen.getByRole("button", { name: "Send message" }))
    expect(await screen.findByText("Research add-on requested")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Run approved research" })).toBeDisabled()
    await user.click(screen.getByRole("checkbox", { name: /approve this objective/i }))
    expect(screen.getByRole("button", { name: "Run approved research" })).toBeEnabled()
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
