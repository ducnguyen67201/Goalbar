import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter, Route, Routes } from "react-router-dom"
import { describe, expect, it } from "vitest"

import { OnboardingFlow } from "./OnboardingFlow"

describe("onboarding", () => {
  it("validates and stores the founder baseline in preview mode", async () => {
    const user = userEvent.setup()
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={["/onboarding"]}>
          <Routes>
            <Route path="/onboarding" element={<OnboardingFlow />} />
            <Route path="/" element={<p>Saved</p>} />
          </Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )
    await user.type(screen.getByLabelText("Your name"), "Duc")
    await user.type(screen.getByLabelText("Product or project"), "Lab")
    await user.type(screen.getByLabelText(/What do you offer\?/), "A sustainable growth system")
    await user.type(
      screen.getByLabelText(/What have you earned the right to talk about\?/),
      "Building local-first products",
    )
    await user.click(screen.getByRole("button", { name: /save founder baseline/i }))
    expect(await screen.findByText("Saved")).toBeInTheDocument()
  })
})
