import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { describe, expect, it } from "vitest"

import App from "./App"

describe("application shell", () => {
  it("renders the local-first Today route", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <App />
        </MemoryRouter>
      </QueryClientProvider>,
    )
    expect(await screen.findByText("Build a growth loop you can trust.")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Goalbar home" })).toBeInTheDocument()
    expect(screen.getByText("Local only")).toBeInTheDocument()
  })

  it("opens the Browser workbench from primary navigation", async () => {
    const user = userEvent.setup()
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <App />
        </MemoryRouter>
      </QueryClientProvider>,
    )
    await user.click(screen.getByRole("link", { name: "Browser" }))
    expect(
      await screen.findByRole("heading", { name: "Chat with the browser beside you." }),
    ).toBeInTheDocument()
    expect(screen.getByRole("region", { name: "Integrated browser" })).toBeInTheDocument()
  })

  it("renders the standalone marketing page and captures a download email locally", async () => {
    const user = userEvent.setup()
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <MemoryRouter initialEntries={["/landing"]}>
          <App />
        </MemoryRouter>
      </QueryClientProvider>,
    )
    expect(screen.getByRole("link", { name: "Goalbar home" })).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Star Goalbar on GitHub, 0 stars" })).toHaveAttribute(
      "href",
      "https://github.com/ducnguyen67201/Goalbar",
    )
    expect(screen.getByRole("heading", { name: "Your GTM cofounder.", level: 1 })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Replay the clicks" })).toBeInTheDocument()
    await user.type(screen.getAllByRole("textbox", { name: "Email address" })[0], "founder@example.com")
    await user.click(screen.getAllByRole("button", { name: "Email me the download" })[0])
    expect(screen.getByText("You’re on the download list.")).toBeInTheDocument()
  })
})
