import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
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
    expect(screen.getByText("Local only")).toBeInTheDocument()
  })
})
