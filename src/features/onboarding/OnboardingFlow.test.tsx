import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter, Route, Routes } from "react-router-dom"
import { describe, expect, it } from "vitest"

import { OnboardingFlow } from "./OnboardingFlow"

describe("onboarding", () => {
  it("creates a starting profile from a founder description", async () => {
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
    await user.type(screen.getByLabelText("Product or company"), "Lab")
    await user.type(screen.getByLabelText(/Describe it in your own words/), "A sustainable growth system")
    await user.type(screen.getByLabelText(/Describe your ICP/), "Solo founders building local-first products")
    await user.click(screen.getByRole("button", { name: /create my starting profile/i }))
    expect(await screen.findByText("Saved")).toBeInTheDocument()
  })

  it("accepts a landing page instead of a written description", async () => {
    const user = userEvent.setup()
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={["/onboarding"]}>
          <Routes>
            <Route path="/onboarding" element={<OnboardingFlow />} />
            <Route path="/" element={<p>Saved from URL</p>} />
          </Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )
    await user.type(screen.getByLabelText(/Paste your landing page/), "https://goalbar.local")
    await user.type(screen.getByLabelText(/Describe your ICP/), "Technical solo founders")
    await user.type(screen.getByLabelText("Your name"), "Duc")
    await user.type(screen.getByLabelText("Product or company"), "Goalbar")
    await user.click(screen.getByRole("button", { name: /create my starting profile/i }))
    expect(await screen.findByText("Saved from URL")).toBeInTheDocument()
  })

  it("asks for a landing page or description before continuing", async () => {
    const user = userEvent.setup()
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={["/onboarding"]}>
          <Routes>
            <Route path="/onboarding" element={<OnboardingFlow />} />
          </Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )
    await user.type(screen.getByLabelText(/Describe your ICP/), "Technical solo founders")
    await user.type(screen.getByLabelText("Your name"), "Duc")
    await user.type(screen.getByLabelText("Product or company"), "Goalbar")
    await user.click(screen.getByRole("button", { name: /create my starting profile/i }))
    expect(await screen.findByText("Add a landing page or a short description.")).toBeVisible()
  })
})
