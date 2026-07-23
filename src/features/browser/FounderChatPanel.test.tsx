import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
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
      <FounderChatPanel
        activeTab={null}
        onNavigate={() => undefined}
        onPrepareReply={() =>
          Promise.resolve({
            status: "prepared",
            platform: "x",
            characterCount: 0,
          })
        }
      />
    </QueryClientProvider>,
  )
}

describe("FounderChatPanel persistence", () => {
  beforeEach(() => {
    mocks.invokeOutput.mockReset()
    mocks.invokeValidated.mockReset()
    mocks.listen.mockReset()
    mocks.listen.mockResolvedValue(() => undefined)
    mocks.invokeOutput.mockImplementation((command: string) => {
      if (command === "list_codex_chats") {
        return Promise.resolve({
          activeThreadId: "thread-1",
          chats: [
            {
              threadId: "thread-1",
              title: "ICP research",
              preview: "Find my ICP",
              createdAt: 1,
              updatedAt: 2,
              status: "idle",
            },
            {
              threadId: "thread-2",
              title: "Content plan",
              preview: "Plan next week",
              createdAt: 1,
              updatedAt: 1,
              status: "idle",
            },
          ],
        })
      }
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
    mocks.invokeValidated.mockImplementation(
      (command: string, payload: { input?: { threadId?: string } }) => {
        if (command === "select_codex_chat") {
          const threadId = payload.input?.threadId
          return Promise.resolve({
            threadId,
            messages:
              threadId === "thread-3"
                ? []
                : threadId === "thread-2"
                  ? [
                      {
                        id: "content-user",
                        role: "user",
                        body: "Plan next week",
                      },
                      {
                        id: "content-assistant",
                        role: "assistant",
                        body: "Here is the content plan.",
                      },
                    ]
                  : [
                      {
                        id: "icp-user",
                        role: "user",
                        body: "Find my ICP",
                      },
                      {
                        id: "icp-assistant",
                        role: "assistant",
                        body: "Let us inspect the evidence.",
                      },
                    ],
          })
        }
        return Promise.resolve({
          threadId: payload.input?.threadId ?? "thread-1",
          turnId: "turn-1",
          reply: "Done",
        })
      },
    )
  })

  it("hydrates the current Codex transcript every time the browser panel mounts", async () => {
    const firstMount = renderChat()
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    expect(screen.getByText("Let us inspect the evidence.")).toBeInTheDocument()

    firstMount.unmount()
    renderChat()

    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    expect(screen.getByText("Let us inspect the evidence.")).toBeInTheDocument()
    expect(mocks.invokeOutput).toHaveBeenCalledTimes(4)
    expect(mocks.invokeOutput).toHaveBeenCalledWith("list_codex_chats", {}, expect.anything())
    expect(mocks.invokeOutput).toHaveBeenCalledWith("get_codex_chat_state", {}, expect.anything())
  })

  it("switches between durable Codex chats and restores each transcript", async () => {
    const user = userEvent.setup()
    renderChat()
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()

    await user.click(screen.getByRole("button", { name: "Content plan" }))

    expect(await screen.findByText("Plan next week")).toBeInTheDocument()
    expect(screen.getByText("Here is the content plan.")).toBeInTheDocument()
    expect(screen.queryByText("Find my ICP")).not.toBeInTheDocument()

    await user.click(screen.getByRole("button", { name: "ICP research" }))

    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    expect(screen.getByText("Let us inspect the evidence.")).toBeInTheDocument()
  })

  it("can switch chats while another Codex turn keeps running", async () => {
    const user = userEvent.setup()
    let finishTurn: ((value: unknown) => void) | undefined
    mocks.invokeValidated.mockImplementation(
      (command: string, payload: { input?: { threadId?: string } }) => {
        if (command === "send_codex_chat_message") {
          return new Promise((resolve) => {
            finishTurn = resolve
          })
        }
        if (command === "select_codex_chat") {
          return Promise.resolve({
            threadId: payload.input?.threadId,
            messages: [
              {
                id: "content-user",
                role: "user",
                body: "Plan next week",
              },
              {
                id: "content-assistant",
                role: "assistant",
                body: "Here is the content plan.",
              },
            ],
          })
        }
        return Promise.resolve(false)
      },
    )
    renderChat()
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    await user.type(screen.getByRole("textbox", { name: "Chat message" }), "Keep researching")
    await user.click(screen.getByRole("button", { name: "Send message" }))

    await user.click(screen.getByRole("button", { name: "Content plan" }))

    expect(await screen.findByText("Here is the content plan.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Send message" })).toBeInTheDocument()
    await user.click(screen.getByRole("button", { name: "ICP research" }))
    expect(await screen.findByRole("button", { name: "Stop Codex response" })).toBeEnabled()

    finishTurn?.({
      threadId: "thread-1",
      turnId: "turn-1",
      reply: "Research complete",
    })
  })

  it("keeps new and saved chats switchable without clearing either transcript", async () => {
    const user = userEvent.setup()
    let newChatCreated = false
    mocks.invokeOutput.mockImplementation((command: string) => {
      if (command === "new_codex_chat") {
        newChatCreated = true
        return Promise.resolve("thread-3")
      }
      if (command === "list_codex_chats") {
        return Promise.resolve({
          activeThreadId: newChatCreated ? "thread-3" : "thread-1",
          chats: [
            ...(newChatCreated
              ? [
                  {
                    threadId: "thread-3",
                    title: "New chat",
                    preview: "",
                    createdAt: 0,
                    updatedAt: 0,
                    status: "idle",
                  },
                ]
              : []),
            {
              threadId: "thread-1",
              title: "ICP research",
              preview: "Find my ICP",
              createdAt: 1,
              updatedAt: 2,
              status: "idle",
            },
            {
              threadId: "thread-2",
              title: "Content plan",
              preview: "Plan next week",
              createdAt: 1,
              updatedAt: 1,
              status: "idle",
            },
          ],
        })
      }
      if (command === "get_codex_chat_state") {
        return Promise.resolve({
          threadId: "thread-1",
          messages: [
            { id: "icp-user", role: "user", body: "Find my ICP" },
            {
              id: "icp-assistant",
              role: "assistant",
              body: "Let us inspect the evidence.",
            },
          ],
        })
      }
      return Promise.resolve(false)
    })
    renderChat()
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    const chatTabs = screen.getByRole("navigation", { name: "Codex chats" })

    await user.click(screen.getByRole("button", { name: "New Codex chat" }))

    const newChatTab = await within(chatTabs).findByRole("button", { name: "New chat" })
    expect(screen.queryByText("Find my ICP")).not.toBeInTheDocument()

    await user.click(within(chatTabs).getByRole("button", { name: "ICP research" }))
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    expect(screen.getByText("Let us inspect the evidence.")).toBeInTheDocument()

    await user.click(newChatTab)
    expect(screen.queryByText("Find my ICP")).not.toBeInTheDocument()
    expect(screen.getByText(/I’m your founder chat/i)).toBeInTheDocument()
  })

  it("starts a second Codex chat while the first chat keeps running", async () => {
    const user = userEvent.setup()
    const finishTurns = new Map<string, (value: unknown) => void>()
    let newChatCreated = false
    mocks.invokeOutput.mockImplementation((command: string) => {
      if (command === "new_codex_chat") {
        newChatCreated = true
        return Promise.resolve("thread-3")
      }
      if (command === "list_codex_chats") {
        return Promise.resolve({
          activeThreadId: newChatCreated ? "thread-3" : "thread-1",
          chats: [
            ...(newChatCreated
              ? [
                  {
                    threadId: "thread-3",
                    title: "New chat",
                    preview: "",
                    createdAt: 0,
                    updatedAt: 0,
                    status: "idle",
                  },
                ]
              : []),
            {
              threadId: "thread-1",
              title: "ICP research",
              preview: "Find my ICP",
              createdAt: 1,
              updatedAt: 2,
              status: "idle",
            },
          ],
        })
      }
      if (command === "get_codex_chat_state") {
        return Promise.resolve({
          threadId: "thread-1",
          messages: [{ id: "icp-user", role: "user", body: "Find my ICP" }],
        })
      }
      return Promise.resolve(false)
    })
    mocks.invokeValidated.mockImplementation(
      (command: string, payload: { input?: { threadId?: string } }) => {
        const threadId = payload.input?.threadId ?? "thread-1"
        if (command === "send_codex_chat_message") {
          return new Promise((resolve) => {
            finishTurns.set(threadId, resolve)
          })
        }
        if (command === "select_codex_chat") {
          return Promise.resolve({
            threadId,
            messages: threadId === "thread-1" ? [{ id: "icp-user", role: "user", body: "Find my ICP" }] : [],
          })
        }
        return Promise.resolve(false)
      },
    )

    renderChat()
    expect(await screen.findByText("Find my ICP")).toBeInTheDocument()
    await user.type(screen.getByRole("textbox", { name: "Chat message" }), "Keep researching")
    await user.click(screen.getByRole("button", { name: "Send message" }))
    await waitFor(() => expect(finishTurns.has("thread-1")).toBe(true))

    await user.click(screen.getByRole("button", { name: "New Codex chat" }))
    const chatTabs = screen.getByRole("navigation", { name: "Codex chats" })
    const newChatTab = await within(chatTabs).findByRole("button", { name: "New chat" })

    expect(screen.getByRole("button", { name: "Send message" })).toBeInTheDocument()
    await user.type(screen.getByRole("textbox", { name: "Chat message" }), "Plan my next post")
    expect(screen.getByRole("button", { name: "Send message" })).toBeEnabled()
    await user.click(screen.getByRole("button", { name: "Send message" }))
    await waitFor(() => expect(finishTurns.has("thread-3")).toBe(true))

    await user.click(within(chatTabs).getByRole("button", { name: "ICP research" }))
    expect(await screen.findByRole("button", { name: "Stop Codex response" })).toBeEnabled()
    await user.click(newChatTab)
    const stopNewChat = await screen.findByRole("button", { name: "Stop Codex response" })
    expect(stopNewChat).toBeEnabled()
    await user.click(stopNewChat)
    expect(mocks.invokeValidated).toHaveBeenCalledWith(
      "interrupt_codex_chat",
      { input: { threadId: "thread-3" } },
      expect.anything(),
      expect.anything(),
    )

    finishTurns.get("thread-1")?.({
      threadId: "thread-1",
      turnId: "turn-1",
      reply: "Research complete",
    })
    finishTurns.get("thread-3")?.({
      threadId: "thread-3",
      turnId: "turn-3",
      reply: "Plan complete",
    })
  })
})
