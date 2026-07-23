import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"

import { bootstrapFixture, founderFixture, historyOverviewFixture } from "@/test/fixtures"

const { invokeValidatedMock, useBootstrapMock } = vi.hoisted(() => ({
  invokeValidatedMock: vi.fn(),
  useBootstrapMock: vi.fn(),
}))

vi.mock("@/app/bootstrap", () => ({ useBootstrap: useBootstrapMock }))
vi.mock("@/lib/tauri", () => ({
  invokeValidated: invokeValidatedMock,
  isTauriRuntime: () => true,
}))

import { MemoryPage } from "./MemoryPage"

const hypothesisId = "2e04a718-5888-41ee-9d07-463fa56a073c"
const revisionId = "8c429d48-d9fb-4df4-abdf-647ee45e8a0c"
const activeHypothesis = {
  id: hypothesisId,
  founderId: founderFixture.id,
  version: 1,
  parentId: null,
  role: "Technical solo SaaS founder",
  situation: "Has early traction and no growth team",
  urgentProblem: "Customer learning is inconsistent",
  currentWorkaround: "Posts ad hoc and checks vanity metrics",
  desiredOutcome: "Repeatable qualified conversations",
  objections: ["Another tool could create busywork"],
  language: ["learning loop", "qualified conversation"],
  confidence: 0.55,
  status: "active",
  createdAt: "2026-07-22T00:00:00Z",
  updatedAt: "2026-07-22T00:00:00Z",
}

function renderMemoryPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  render(
    <QueryClientProvider client={client}>
      <MemoryPage />
    </QueryClientProvider>,
  )
}

describe("MemoryPage", () => {
  beforeEach(() => {
    useBootstrapMock.mockReturnValue({ data: bootstrapFixture })
    invokeValidatedMock.mockReset()
    invokeValidatedMock.mockImplementation((command: string) => {
      switch (command) {
        case "get_history_overview":
          return Promise.resolve(historyOverviewFixture)
        case "list_icp_hypotheses":
          return Promise.resolve([activeHypothesis])
        case "update_founder_profile":
          return Promise.resolve(founderFixture)
        case "revise_icp_hypothesis":
          return Promise.resolve(revisionId)
        case "generate_icp_hypotheses":
          return Promise.resolve({ hypotheses: [activeHypothesis] })
        default:
          return Promise.resolve(null)
      }
    })
  })

  it("edits the founder baseline without replacing its local identity", async () => {
    const user = userEvent.setup()
    renderMemoryPage()

    await user.click(screen.getByRole("button", { name: "Edit founder baseline" }))
    const idealCustomer = screen.getByLabelText("Current ideal customer")
    await user.clear(idealCustomer)
    await user.type(idealCustomer, "AI-native founders with early customer pull")
    await user.type(screen.getByLabelText("Goals, one per line"), "{enter}Founder referrals")
    await user.click(screen.getByRole("button", { name: "Save baseline" }))

    await waitFor(() => {
      const updateCall = invokeValidatedMock.mock.calls.find(
        ([command]) => command === "update_founder_profile",
      )
      expect(updateCall?.[1]).toMatchObject({
        input: {
          founderId: founderFixture.id,
          profile: {
            idealCustomer: "AI-native founders with early customer pull",
            goals: ["Qualified conversations", "Founder referrals"],
          },
        },
      })
    })
  })

  it("saves manual ICP changes as a child revision", async () => {
    const user = userEvent.setup()
    renderMemoryPage()

    await user.click(await screen.findByRole("button", { name: /edit into a new version/i }))
    const role = screen.getByLabelText("Who this customer is")
    await user.clear(role)
    await user.type(role, "AI-native technical founder")
    await user.click(screen.getByRole("button", { name: "Save as proposed version" }))

    await waitFor(() => {
      const revisionCall = invokeValidatedMock.mock.calls.find(
        ([command]) => command === "revise_icp_hypothesis",
      )
      expect(revisionCall?.[1]).toMatchObject({
        input: {
          hypothesisId,
          hypothesis: { role: "AI-native technical founder" },
        },
      })
    })
  })

  it("refreshes adaptive ICP memory through the installed Codex CLI", async () => {
    const user = userEvent.setup()
    useBootstrapMock.mockReturnValue({
      data: {
        ...bootstrapFixture,
        agents: [{ provider: "claude", readiness: "ready", version: "preview" }, ...bootstrapFixture.agents],
      },
    })
    renderMemoryPage()

    await user.click(await screen.findByRole("button", { name: "Refresh with Codex" }))

    await waitFor(() => {
      const refreshCall = invokeValidatedMock.mock.calls.find(
        ([command]) => command === "generate_icp_hypotheses",
      )
      expect(refreshCall?.[1]).toEqual({ input: { provider: "codex" } })
    })
  })
})
