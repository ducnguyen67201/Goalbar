import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"

const mocks = vi.hoisted(() => ({
  invokeOutput: vi.fn(),
  invokeValidated: vi.fn(),
  listen: vi.fn(),
}))

vi.mock("@/app/bootstrap", () => ({
  useBootstrap: () => ({
    data: {
      agents: [
        { provider: "codex", readiness: "ready" },
        { provider: "claude", readiness: "ready" },
      ],
    },
  }),
}))

vi.mock("@/lib/tauri", () => ({
  isTauriRuntime: () => true,
  invokeOutput: mocks.invokeOutput,
  invokeValidated: mocks.invokeValidated,
}))

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}))

import { FounderChatPanel } from "./FounderChatPanel"

function renderChat() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <FounderChatPanel activeTab={null} />
    </QueryClientProvider>,
  )
}

describe("FounderChatPanel persistence", () => {
  beforeEach(() => {
    mocks.listen.mockResolvedValue(() => undefined)
    mocks.invokeOutput.mockImplementation((command: string) => {
      if (command === "get_codex_chat_state") {
        return Promise.resolve({
          threadId: "thread-1",
          messages: [
            {
              id: "b9d7afe0-1807-4ad9-bf22-2945f0bb9081",
              role: "user",
              body: "Find my ICP",
            },
            {
              id: "2e5745e4-1aaf-4e8a-86a6-5e5de8245daa",
              role: "assistant",
              body: "Let us inspect the evidence.",
            },
          ],
        })
      }
      return Promise.resolve(false)
    })
  })

  it("hydrates the current Codex transcript every time the browser panel mounts", async () => {
    const firstMount = renderChat()
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    expect(screen.getByText("Let us inspect the evidence.")).toBeInTheDocument()

    firstMount.unmount()
    renderChat()

    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    expect(screen.getByText("Let us inspect the evidence.")).toBeInTheDocument()
    expect(mocks.invokeOutput).toHaveBeenCalledTimes(2)
    expect(mocks.invokeOutput).toHaveBeenCalledWith("get_codex_chat_state", {}, expect.anything())
  })
})
