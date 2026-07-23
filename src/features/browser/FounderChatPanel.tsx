import { useMutation } from "@tanstack/react-query"
import { listen } from "@tauri-apps/api/event"
import {
  Archive,
  Bot,
  CheckCircle2,
  FlaskConical,
  ListChecks,
  LoaderCircle,
  MessageCircle,
  Plus,
  Send,
  Sparkles,
  Square,
  XCircle,
} from "lucide-react"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { z } from "zod"

import { useBootstrap } from "@/app/bootstrap"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { EngagementSuggestionCard } from "@/features/browser/EngagementSuggestionCard"
import { parseEngagementSuggestion } from "@/features/browser/engagement-suggestion"
import { HistoryImportPanel } from "@/features/browser/HistoryImportPanel"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  codexChatCollectionSchema,
  codexChatEventSchema,
  codexChatStateSchema,
  codexChatTurnResultSchema,
  founderChatAgentResultSchema,
  founderChatOutputJsonSchema,
  founderChatResearchRequestSchema,
  founderChatTurnSchema,
  interruptCodexChatInputSchema,
  runAgentTaskInputSchema,
  selectCodexChatInputSchema,
  sendCodexChatInputSchema,
  type AgentProvider,
  type CodexChatEvent,
  type CodexChatSummary,
  type EngagementSuggestion,
  type FounderChatResearchRequest,
  type FounderChatTurn,
} from "@/schemas/agent"
import {
  browserResearchTraceSchema,
  browserRunProgressSchema,
  cancelBrowserCollectionInputSchema,
  reviewBrowserResearchFindingInputSchema,
  startBrowserCollectionInputSchema,
  storedResearchFindingSchema,
  type BrowserResearchTrace,
  type BrowserReplyPreparation,
  type BrowserRunProgress,
  type BrowserTab,
  type StoredResearchFinding,
} from "@/schemas/browser"

type FounderChatPanelProps = {
  activeTab: BrowserTab | null
  onNavigate: (url: string) => void
  onPrepareReply: (url: string, reply: string) => Promise<BrowserReplyPreparation>
}

type ChatMessage = {
  id: string
  role: "assistant" | "user" | "tool"
  body: string
}

type BrowserTaskInvocation = {
  request: FounderChatResearchRequest
  tab: BrowserTab
  provider: AgentProvider
}

type ChatSubmission = {
  message: string
  displayMessage?: string
  tab: BrowserTab | null
  provider: AgentProvider
}

type CodexChatSubmission = {
  threadId: string
  message: string
  displayMessage?: string
  tab: BrowserTab | null
}

type CodexToolActivity = {
  tool: string
  status: "running" | "completed" | "paused"
  message: string | null
}

const AUTOMATIC_BROWSER_MAX_ITEMS = 25
const AUTOMATIC_BROWSER_MAX_STEPS = 8
const PREVIEW_CODEX_CHAT_ID = "preview-codex-chat"

const previewCodexChat: CodexChatSummary = {
  threadId: PREVIEW_CODEX_CHAT_ID,
  title: "New chat",
  preview: "",
  createdAt: 0,
  updatedAt: 0,
  status: "idle",
}

const initialMessage: ChatMessage = {
  id: "founder-chat-welcome",
  role: "assistant",
  body: "I’m your founder chat. Ask about your ICP, positioning, content, or the page beside us. When your request needs browser evidence, I’ll use Browser Use on the open supported tab.",
}

const chatPrompt = `
You are Goalbar's founder chat: a concise strategic partner for a solo founder.
Help with ICP discovery, founder voice, content, conversations, and sustainable growth.
You can request one local tool named browser_use_current_tab when the answer requires evidence from the
currently visible X, LinkedIn, or Reddit page. Never claim you read the page unless that tool has run.
Use it for requests to find, rank, compare, summarize, or analyze posts, accounts, profiles, feeds,
audience language, or ICP signals on the current page. The user's message authorizes this bounded,
read-only tool call, so do not ask for a second confirmation. When the user gives a count, use it as
maximumItems. Otherwise default to 12 items. Never request more than 25 items or 8 browser steps.
When browser evidence is useful, return a researchRequest with a focused objective, a short reason,
ownership, and conservative limits. Otherwise researchRequest must be null.
Never publish, send a DM, or imply an external action occurred.
When you recommend one specific post to engage with, include this exact machine-readable block at the
end of the reply, with valid JSON and no markdown inside the JSON:
<goalbar-engagement>
{"title":"Short post title","url":"https://exact-post-url","reason":"Why this is the best next move","reply":"The exact suggested reply"}
</goalbar-engagement>
Only include the block when all four fields are grounded and useful. The app turns it into an editable
action card; it still never posts automatically.
`.trim()

function newMessage(role: ChatMessage["role"], body: string): ChatMessage {
  return { id: crypto.randomUUID(), role, body }
}

function boundAutomaticResearchRequest(request: FounderChatResearchRequest): FounderChatResearchRequest {
  return founderChatResearchRequestSchema.parse({
    ...request,
    maximumItems: Math.min(request.maximumItems, AUTOMATIC_BROWSER_MAX_ITEMS),
    maximumSteps: Math.min(request.maximumSteps, AUTOMATIC_BROWSER_MAX_STEPS),
  })
}

function requestedItemCount(message: string): number | null {
  const match = message.match(
    /\b(?:(?:top|good|best|relevant)\s+)*(\d{1,3})\s+(?:(?:good|best|relevant)\s+)*(?:posts?|accounts?|profiles?|items?|examples?)\b/i,
  )
  if (!match) return null
  return Math.max(1, Math.min(Number(match[1]), AUTOMATIC_BROWSER_MAX_ITEMS))
}

function inferBrowserResearchRequest(message: string): FounderChatResearchRequest | null {
  const hasBrowserVerb =
    /\b(research|analy[sz]e|find|discover|scan|browse|explore|look|open|summari[sz]e|rank|compare)\b/i.test(
      message,
    )
  const hasBrowserObject =
    /\b(icp|audience|customers?|signals?|pains?|posts?|accounts?|profiles?|feeds?|pages?|tabs?)\b/i.test(
      message,
    )
  const pointsAtOpenPage =
    /\b(?:this|current|visible|open)\s+(?:page|feed|tab|profile|post|account)\b|\bon (?:this|the) (?:page|screen)\b/i.test(
      message,
    )
  const wantsResearch = (hasBrowserVerb && hasBrowserObject) || pointsAtOpenPage
  if (!wantsResearch) return null
  const maximumItems = requestedItemCount(message) ?? 12
  return founderChatResearchRequestSchema.parse({
    objective: message.trim(),
    reason: "The answer depends on navigating and reading evidence in the open browser tab.",
    ownership: "reference",
    maximumItems,
    maximumSteps: Math.min(AUTOMATIC_BROWSER_MAX_STEPS, Math.max(2, Math.ceil(maximumItems / 5) + 1)),
  })
}

function previewFounderChatTurn(message: string): FounderChatTurn {
  const researchRequest = inferBrowserResearchRequest(message)
  return founderChatTurnSchema.parse({
    reply: researchRequest
      ? "I’ll use Browser Use on the open tab now and return a bounded, grounded result."
      : "I can help shape that. Give me the audience, the outcome you want, and what you already believe to be true.",
    researchRequest,
  })
}

function previewEngagementReply(platform: NonNullable<BrowserTab["platform"]>) {
  const details = {
    x: {
      title: "A founder sharing the messy story behind their product",
      url: "https://x.com/goalbar/status/1234567890123456789",
    },
    linkedin: {
      title: "A solo founder explaining what finally unlocked early growth",
      url: "https://www.linkedin.com/posts/goalbar_founder-growth-activity-1234567890123456789",
    },
    reddit: {
      title: "A candid build-in-public lesson from an early-stage founder",
      url: "https://www.reddit.com/r/startups/comments/goalbar/founder_lesson/",
    },
  }[platform]
  return `
I found one focused next move.
<goalbar-engagement>
${JSON.stringify({
  ...details,
  reason:
    "It directly overlaps with your founder-voice theme and still has room for a specific, thoughtful response.",
  reply:
    "The most useful founder stories usually start before the startup—with the frustration or failed attempt that made the problem impossible to ignore. That detail builds more trust than a polished origin story ever could.",
})}
</goalbar-engagement>
  `.trim()
}

function browserToolLabel(tool: string) {
  switch (tool) {
    case "browser_observe":
      return "Reading the visible page"
    case "browser_scroll":
      return "Moving through the page"
    case "browser_scan_feed":
      return "Scanning consecutive feed batches"
    case "browser_open_link":
      return "Opening a visible link"
    case "browser_go_back":
      return "Returning to the starting page"
    default:
      return tool
  }
}

export function FounderChatPanel({ activeTab, onNavigate, onPrepareReply }: FounderChatPanelProps) {
  const bootstrap = useBootstrap()
  const [provider, setProvider] = useState<AgentProvider>("codex")
  const [codexChats, setCodexChats] = useState<CodexChatSummary[]>([previewCodexChat])
  const [activeChatId, setActiveChatId] = useState(PREVIEW_CODEX_CHAT_ID)
  const [messages, setMessages] = useState<ChatMessage[]>([initialMessage])
  const [composer, setComposer] = useState("")
  const [researchRequest, setResearchRequest] = useState<FounderChatResearchRequest | null>(null)
  const [researchTab, setResearchTab] = useState<BrowserTab | null>(null)
  const [progress, setProgress] = useState<BrowserRunProgress | null>(null)
  const [trace, setTrace] = useState<BrowserResearchTrace[]>([])
  const [findings, setFindings] = useState<StoredResearchFinding[]>([])
  const [historyOpen, setHistoryOpen] = useState(false)
  const [streamingReplies, setStreamingReplies] = useState<Map<string, string>>(() => new Map())
  const [codexToolActivity, setCodexToolActivity] = useState<CodexToolActivity | null>(null)
  const [chatStateError, setChatStateError] = useState<Error | null>(null)
  const [pendingThreadIds, setPendingThreadIds] = useState<Set<string>>(() => new Set())
  const activeChatIdRef = useRef(PREVIEW_CODEX_CHAT_ID)
  const pendingThreadIdsRef = useRef(new Set<string>())
  const transcriptsRef = useRef(new Map<string, ChatMessage[]>())

  const agentStatuses = useMemo(
    () => new Map(bootstrap.data?.agents.map((status) => [status.provider, status])),
    [bootstrap.data?.agents],
  )
  const activeCodexChat = codexChats.find((chat) => chat.threadId === activeChatId) ?? null
  const streamingReply = streamingReplies.get(activeChatId) ?? ""

  const resetChatSurface = useCallback(() => {
    setComposer("")
    setCodexToolActivity(null)
    setResearchRequest(null)
    setResearchTab(null)
    setProgress(null)
    setTrace([])
    setFindings([])
  }, [])

  const hydrateCodexChat = useCallback(async () => {
    if (!isTauriRuntime()) return
    try {
      const state = await invokeOutput("get_codex_chat_state", {}, codexChatStateSchema)
      if (state.threadId !== activeChatIdRef.current) return
      transcriptsRef.current.set(state.threadId, state.messages)
      setMessages([initialMessage, ...state.messages])
      setChatStateError(null)
    } catch (error) {
      setChatStateError(error instanceof Error ? error : new Error(String(error)))
    }
  }, [])

  const refreshCodexChats = useCallback(async () => {
    if (!isTauriRuntime()) return
    try {
      const collection = await invokeOutput("list_codex_chats", {}, codexChatCollectionSchema)
      setCodexChats(
        collection.chats.map((chat) =>
          pendingThreadIdsRef.current.has(chat.threadId) ? { ...chat, status: "active" } : chat,
        ),
      )
      setChatStateError(null)
    } catch (error) {
      setChatStateError(error instanceof Error ? error : new Error(String(error)))
    }
  }, [])

  useEffect(() => {
    if (!isTauriRuntime()) return
    const hydrationTimer = window.setTimeout(() => {
      void (async () => {
        try {
          const collection = await invokeOutput("list_codex_chats", {}, codexChatCollectionSchema)
          activeChatIdRef.current = collection.activeThreadId
          setActiveChatId(collection.activeThreadId)
          setCodexChats(
            collection.chats.map((chat) =>
              pendingThreadIdsRef.current.has(chat.threadId) ? { ...chat, status: "active" } : chat,
            ),
          )
          const state = await invokeOutput("get_codex_chat_state", {}, codexChatStateSchema)
          if (state.threadId === collection.activeThreadId) {
            transcriptsRef.current.set(state.threadId, state.messages)
            setMessages([initialMessage, ...state.messages])
          }
          setChatStateError(null)
        } catch (error) {
          setChatStateError(error instanceof Error ? error : new Error(String(error)))
        }
      })()
    }, 0)
    const progressListener = listen<unknown>("browser://run-progress", (event) => {
      setProgress(browserRunProgressSchema.parse(event.payload))
    })
    const traceListener = listen<unknown>("browser://research-trace", (event) => {
      const item = browserResearchTraceSchema.parse(event.payload)
      setTrace((current) =>
        current.some((existing) => existing.id === item.id) ? current : [...current, item],
      )
    })
    const findingListener = listen<unknown>("browser://research-finding", (event) => {
      const item = storedResearchFindingSchema.parse(event.payload)
      setFindings((current) =>
        current.some((existing) => existing.id === item.id) ? current : [item, ...current],
      )
    })
    const codexChatListener = listen<unknown>("codex://chat-event", (event) => {
      const item: CodexChatEvent = codexChatEventSchema.parse(event.payload)
      if (item.kind === "turn_started") {
        setCodexChats((current) =>
          current.map((chat) => (chat.threadId === item.threadId ? { ...chat, status: "active" } : chat)),
        )
      }
      if (item.kind === "assistant_delta" && item.delta) {
        setStreamingReplies((current) => {
          const next = new Map(current)
          next.set(item.threadId, (next.get(item.threadId) ?? "") + item.delta)
          return next
        })
      }
      if (item.threadId !== activeChatIdRef.current) {
        if (item.kind === "turn_completed" || item.kind === "state_changed") {
          void refreshCodexChats()
        }
        return
      }
      if (item.kind === "tool_started" && item.tool) {
        setCodexToolActivity({ tool: item.tool, status: "running", message: null })
      } else if (item.kind === "tool_completed" && item.tool) {
        setCodexToolActivity({
          tool: item.tool,
          status: item.success === false ? "paused" : "completed",
          message: item.message,
        })
      } else if (item.kind === "state_changed") {
        void refreshCodexChats()
        if (!pendingThreadIdsRef.current.has(item.threadId)) void hydrateCodexChat()
      }
    })
    return () => {
      window.clearTimeout(hydrationTimer)
      void progressListener.then((dispose) => dispose())
      void traceListener.then((dispose) => dispose())
      void findingListener.then((dispose) => dispose())
      void codexChatListener.then((dispose) => dispose())
    }
  }, [hydrateCodexChat, refreshCodexChats])

  useEffect(() => {
    transcriptsRef.current.set(activeChatId, messages.slice(1))
  }, [activeChatId, messages])

  const loadResearchArtifacts = async (runId: string) => {
    if (!isTauriRuntime()) return
    const input = { runId }
    const [storedFindings, storedTrace] = await Promise.all([
      invokeValidated(
        "list_browser_research_findings",
        { input },
        cancelBrowserCollectionInputSchema,
        z.array(storedResearchFindingSchema),
      ),
      invokeValidated(
        "list_browser_research_trace",
        { input },
        cancelBrowserCollectionInputSchema,
        z.array(browserResearchTraceSchema),
      ),
    ])
    setFindings(storedFindings)
    setTrace(storedTrace)
  }

  const collect = useMutation({
    mutationFn: async ({ request, tab, provider: taskProvider }: BrowserTaskInvocation) => {
      const input = {
        tabId: tab.id,
        objective: request.objective,
        limits: {
          maximumItems: request.maximumItems,
          maximumSteps: request.maximumSteps,
        },
        ownership: request.ownership,
        provider: taskProvider,
      }
      if (!isTauriRuntime())
        return browserRunProgressSchema.parse({
          runId: crypto.randomUUID(),
          status: "completed",
          step: 1,
          itemCount: request.maximumItems,
          newItemCount: request.maximumItems,
          pauseReason: null,
          summary: "Preview Browser Use completed with a bounded sample.",
        })
      return invokeValidated(
        "start_browser_collection",
        { input },
        startBrowserCollectionInputSchema,
        browserRunProgressSchema,
      )
    },
    onMutate: ({ request, tab }) => {
      setResearchRequest(request)
      setResearchTab(tab)
      setProgress(null)
      setTrace([])
      setFindings([])
    },
    onSuccess: (value) => {
      setProgress(value)
      setMessages((current) => [
        ...current,
        newMessage(
          "tool",
          value.summary ??
            `Research ${value.status}: ${value.itemCount} items inspected in ${value.step} steps.`,
        ),
      ])
      void loadResearchArtifacts(value.runId)
    },
  })

  const codexChat = useMutation({
    mutationFn: async ({ threadId, message, tab }: CodexChatSubmission) => {
      if (!isTauriRuntime()) {
        const browserRequest = inferBrowserResearchRequest(message)
        const wantsEngagement =
          /\b(?:engage|comment|reply)\b[\s\S]{0,80}\b(?:post|thread)\b|\b(?:post|thread)\b[\s\S]{0,80}\b(?:engage|comment|reply)\b/i.test(
            message,
          )
        return codexChatTurnResultSchema.parse({
          threadId,
          turnId: crypto.randomUUID(),
          reply: browserRequest
            ? tab?.platform
              ? wantsEngagement
                ? previewEngagementReply(tab.platform)
                : `I inspected the open ${tab.platform.toUpperCase()} page with Browser Use and prepared a grounded answer.`
              : "Open X, LinkedIn, or Reddit beside this chat so Browser Use has a page to inspect."
            : previewFounderChatTurn(message).reply,
        })
      }
      const input = {
        threadId,
        message,
        activeTabId: tab?.platform ? tab.id : null,
      }
      return invokeValidated(
        "send_codex_chat_message",
        { input },
        sendCodexChatInputSchema,
        codexChatTurnResultSchema,
      )
    },
    onMutate: ({ threadId, message, displayMessage, tab }) => {
      pendingThreadIdsRef.current.add(threadId)
      setPendingThreadIds(new Set(pendingThreadIdsRef.current))
      setCodexChats((current) =>
        current.map((chat) => (chat.threadId === threadId ? { ...chat, status: "active" } : chat)),
      )
      setMessages((current) => {
        const next = [...current, newMessage("user", displayMessage ?? message)]
        transcriptsRef.current.set(threadId, next.slice(1))
        return next
      })
      setComposer("")
      setStreamingReplies((current) => {
        const next = new Map(current)
        next.delete(threadId)
        return next
      })
      setCodexToolActivity(
        !isTauriRuntime() && inferBrowserResearchRequest(message) && tab?.platform
          ? { tool: "browser_observe", status: "running", message: null }
          : null,
      )
      setResearchRequest(null)
      setResearchTab(null)
      setProgress(null)
      setTrace([])
      setFindings([])
    },
    onSuccess: (result, submission) => {
      const transcript = [
        ...(transcriptsRef.current.get(result.threadId) ?? []),
        newMessage("assistant", result.reply),
      ]
      transcriptsRef.current.set(result.threadId, transcript)
      if (result.threadId === activeChatIdRef.current) setMessages([initialMessage, ...transcript])
      setStreamingReplies((current) => {
        const next = new Map(current)
        next.delete(result.threadId)
        return next
      })
      if (result.threadId === activeChatIdRef.current) {
        setCodexToolActivity((current) =>
          current?.status === "running"
            ? {
                ...current,
                status: "completed",
                message: current.message ?? "Browser action completed",
              }
            : current,
        )
      }
      if (!isTauriRuntime()) {
        setCodexChats((current) =>
          current.map((chat) =>
            chat.threadId === result.threadId
              ? {
                  ...chat,
                  title:
                    chat.title === "New chat"
                      ? (submission.displayMessage ?? submission.message).slice(0, 64)
                      : chat.title,
                  status: "idle",
                  updatedAt: Date.now(),
                }
              : chat,
          ),
        )
      }
    },
    onError: (error, submission) => {
      if (submission.threadId === activeChatIdRef.current) {
        setCodexToolActivity((current) =>
          current?.status === "running" ? { ...current, status: "paused", message: error.message } : current,
        )
      }
      setStreamingReplies((current) => {
        const next = new Map(current)
        next.delete(submission.threadId)
        return next
      })
    },
    onSettled: (_result, _error, submission) => {
      pendingThreadIdsRef.current.delete(submission.threadId)
      setPendingThreadIds(new Set(pendingThreadIdsRef.current))
      if (isTauriRuntime()) void refreshCodexChats()
    },
  })

  const chat = useMutation({
    mutationFn: async ({ message, tab, provider: chatProvider }: ChatSubmission) => {
      if (!isTauriRuntime()) return previewFounderChatTurn(message)
      const input = {
        provider: chatProvider,
        taskKind: "founder_chat",
        prompt: chatPrompt,
        context: {
          conversation: [...messages, newMessage("user", message)]
            .slice(-12)
            .map(({ role, body }) => ({ role, body })),
          activePage: tab
            ? {
                title: tab.title,
                url: tab.currentUrl,
                platform: tab.platform ?? null,
              }
            : null,
          tools: [
            {
              name: "browser_use_current_tab",
              description:
                "Automatically observe, scroll, follow visible same-platform links, and go back in the current supported tab. It pauses before typing, arbitrary clicks, publishing, messaging, liking, following, or any account state change.",
            },
          ],
        },
        outputSchema: founderChatOutputJsonSchema,
      }
      const result = await invokeValidated(
        "run_agent_task",
        { input },
        runAgentTaskInputSchema,
        founderChatAgentResultSchema,
      )
      return result.output
    },
    onMutate: ({ message, displayMessage }) => {
      setMessages((current) => [...current, newMessage("user", displayMessage ?? message)])
      setComposer("")
    },
    onSuccess: (turn, submission) => {
      setMessages((current) => [...current, newMessage("assistant", turn.reply)])
      setProgress(null)
      setTrace([])
      setFindings([])
      if (!turn.researchRequest) {
        setResearchRequest(null)
        setResearchTab(null)
        return
      }

      const request = boundAutomaticResearchRequest(turn.researchRequest)
      setResearchRequest(request)
      setResearchTab(submission.tab)
      if (!submission.tab?.platform) {
        setMessages((current) => [
          ...current,
          newMessage("tool", "Browser Use paused: open an X, LinkedIn, or Reddit page, then ask me again."),
        ])
        return
      }
      collect.mutate({ request, tab: submission.tab, provider: submission.provider })
    },
  })

  const interruptCodexChat = useMutation({
    mutationFn: async (threadId: string) => {
      if (!isTauriRuntime()) return true
      const input = { threadId }
      return invokeValidated("interrupt_codex_chat", { input }, interruptCodexChatInputSchema, z.boolean())
    },
  })

  const selectCodexChat = useMutation({
    mutationFn: async (threadId: string) => {
      if (!isTauriRuntime()) {
        return codexChatStateSchema.parse({
          threadId,
          messages: transcriptsRef.current.get(threadId) ?? [],
        })
      }
      const input = { threadId }
      return invokeValidated("select_codex_chat", { input }, selectCodexChatInputSchema, codexChatStateSchema)
    },
    onSuccess: (state, threadId) => {
      const selectedThreadId = state.threadId ?? threadId
      const transcript =
        pendingThreadIdsRef.current.has(selectedThreadId) && transcriptsRef.current.has(selectedThreadId)
          ? (transcriptsRef.current.get(selectedThreadId) ?? [])
          : state.messages
      transcriptsRef.current.set(selectedThreadId, transcript)
      activeChatIdRef.current = selectedThreadId
      setActiveChatId(selectedThreadId)
      setMessages([initialMessage, ...transcript])
      resetChatSurface()
    },
  })

  const newCodexChat = useMutation({
    mutationFn: async () => {
      if (!isTauriRuntime()) return crypto.randomUUID()
      return invokeOutput("new_codex_chat", {}, z.string().min(1))
    },
    onSuccess: (threadId) => {
      activeChatIdRef.current = threadId
      setActiveChatId(threadId)
      transcriptsRef.current.set(threadId, [])
      if (isTauriRuntime()) {
        void refreshCodexChats()
      } else {
        setCodexChats((current) => [
          {
            ...previewCodexChat,
            threadId,
            createdAt: Date.now(),
            updatedAt: Date.now(),
          },
          ...current,
        ])
      }
      setMessages([initialMessage])
      resetChatSurface()
    },
  })

  const review = useMutation({
    mutationFn: async (input: { findingId: string; status: "accepted" | "rejected" }) => {
      if (!isTauriRuntime()) {
        const finding = findings.find((candidate) => candidate.id === input.findingId)
        if (!finding) throw new Error("Finding not found")
        return storedResearchFindingSchema.parse({
          ...finding,
          status: input.status,
          updatedAt: new Date().toISOString(),
        })
      }
      return invokeValidated(
        "review_browser_research_finding",
        { input },
        reviewBrowserResearchFindingInputSchema,
        storedResearchFindingSchema,
      )
    },
    onSuccess: (updated) =>
      setFindings((current) => current.map((finding) => (finding.id === updated.id ? updated : finding))),
  })

  const submitMessage = (rawMessage: string) => {
    const message = rawMessage.trim()
    const selectedCodexChatIsRunning =
      pendingThreadIdsRef.current.has(activeChatId) || activeCodexChat?.status === "active"
    if (!message || chat.isPending || selectedCodexChatIsRunning || collect.isPending) return

    if (provider === "codex") {
      codexChat.mutate({ threadId: activeChatId, message, tab: activeTab })
      return
    }

    const inferredRequest = inferBrowserResearchRequest(message)
    if (!inferredRequest) {
      chat.mutate({ message, tab: activeTab, provider })
      return
    }

    const request = boundAutomaticResearchRequest(inferredRequest)
    const response = activeTab?.platform
      ? `I’ll use Browser Use on the open ${activeTab.platform.toUpperCase()} tab now and return a bounded, grounded result.`
      : "Browser Use needs an open X, LinkedIn, or Reddit tab."
    setMessages((current) => [
      ...current,
      newMessage("user", message),
      newMessage("assistant", response),
      ...(!activeTab?.platform
        ? [newMessage("tool", "Browser Use paused: open an X, LinkedIn, or Reddit page, then ask me again.")]
        : []),
    ])
    setComposer("")
    setResearchRequest(request)
    setResearchTab(activeTab)
    setProgress(null)
    setTrace([])
    setFindings([])
    if (activeTab?.platform) collect.mutate({ request, tab: activeTab, provider })
  }

  const submit = () => submitMessage(composer)

  const rewriteEngagement = (suggestion: EngagementSuggestion) => {
    const message = `
Rewrite this suggested reply so it sounds natural, specific, and like a thoughtful founder—not a
generic social media comment. Preserve the recommendation and return the result as a Goalbar
engagement card.

Post: ${suggestion.title}
URL: ${suggestion.url}
Why it fits: ${suggestion.reason}
Current reply:
${suggestion.reply}
    `.trim()

    if (provider === "codex") {
      codexChat.mutate({
        threadId: activeChatId,
        message,
        displayMessage: "Rewrite the suggested reply",
        tab: activeTab,
      })
      return
    }

    chat.mutate({
      message,
      displayMessage: "Rewrite the suggested reply",
      tab: activeTab,
      provider,
    })
  }

  const error =
    codexChat.error ??
    chat.error ??
    collect.error ??
    interruptCodexChat.error ??
    selectCodexChat.error ??
    newCodexChat.error ??
    review.error ??
    chatStateError
  const anyCodexChatRunning =
    pendingThreadIds.size > 0 || codexChats.some((chatItem) => chatItem.status === "active")
  const selectedCodexChatRunning = pendingThreadIds.has(activeChatId) || activeCodexChat?.status === "active"
  const chatPending = provider === "codex" ? selectedCodexChatRunning : chat.isPending
  const conversationStarted = messages.some((message) => message.role === "user")

  return (
    <section className="founder-chat" aria-label="Founder chat">
      <header className="founder-chat-header">
        <div>
          <span className="chat-agent-mark">
            <Bot size={15} />
          </span>
          <div>
            <strong>
              {provider === "codex" && activeCodexChat?.title !== "New chat"
                ? activeCodexChat?.title
                : "Founder chat"}
            </strong>
            <small>
              Local {provider} session
              {provider === "codex"
                ? ` · ${codexChats.length} chat${codexChats.length === 1 ? "" : "s"}`
                : ""}
            </small>
          </div>
        </div>
        <div className="founder-chat-header-actions">
          <div className="segmented compact" aria-label="Chat provider">
            {(["codex", "claude"] as const).map((candidate) => {
              const status = agentStatuses.get(candidate)
              const unavailable = Boolean(status && status.readiness !== "ready")
              return (
                <button
                  className={provider === candidate ? "active" : ""}
                  disabled={unavailable || chat.isPending || anyCodexChatRunning}
                  key={candidate}
                  onClick={() => setProvider(candidate)}
                >
                  {candidate}
                </button>
              )
            })}
          </div>
          <button
            className="chat-new-button"
            type="button"
            aria-label="New Codex chat"
            disabled={newCodexChat.isPending || selectCodexChat.isPending}
            onClick={() => newCodexChat.mutate()}
          >
            {newCodexChat.isPending ? <LoaderCircle size={13} /> : <Plus size={13} />}
          </button>
        </div>
      </header>

      {provider === "codex" && (
        <nav className="founder-chat-tabs" aria-label="Codex chats">
          {codexChats.map((chatItem) => (
            <button
              type="button"
              className={chatItem.threadId === activeChatId ? "active" : ""}
              aria-pressed={chatItem.threadId === activeChatId}
              disabled={selectCodexChat.isPending}
              key={chatItem.threadId}
              onClick={() => {
                if (chatItem.threadId !== activeChatId) selectCodexChat.mutate(chatItem.threadId)
              }}
            >
              <span className={`chat-tab-status ${chatItem.status}`} aria-hidden="true" />
              <span>{chatItem.title}</span>
            </button>
          ))}
        </nav>
      )}

      <div className="founder-chat-messages" aria-live="polite">
        {messages.map((message) => {
          const engagement = message.role === "assistant" ? parseEngagementSuggestion(message.body) : null
          return (
            <article className={`chat-message ${message.role}`} key={message.id}>
              <span>{message.role === "user" ? "You" : message.role === "tool" ? "Browser" : "Goalbar"}</span>
              {engagement ? (
                <EngagementSuggestionCard
                  suggestion={engagement}
                  rewritePending={chatPending}
                  onOpen={onNavigate}
                  onPrepare={onPrepareReply}
                  onRewrite={rewriteEngagement}
                />
              ) : (
                <p>{message.body}</p>
              )}
            </article>
          )
        })}
        {!conversationStarted && !chatPending && (
          <section className="chat-quick-actions" aria-label="Suggested actions">
            <div>
              <Sparkles size={13} />
              <span>
                <strong>Start with one click</strong>
                <small>Goalbar will use the open page when it needs evidence.</small>
              </span>
            </div>
            <button
              type="button"
              onClick={() =>
                submitMessage(
                  "Find one high-fit post I can thoughtfully engage with. Recommend the best one and draft a reply in my voice.",
                )
              }
            >
              <MessageCircle size={13} />
              <span>
                <strong>Find a post to engage</strong>
                <small>Choose one and draft my reply</small>
              </span>
            </button>
            <button
              type="button"
              onClick={() =>
                submitMessage(
                  "Give me the single best growth action for today using what you know about my ICP and founder voice.",
                )
              }
            >
              <ListChecks size={13} />
              <span>
                <strong>Recommend today’s action</strong>
                <small>Keep it focused and realistic</small>
              </span>
            </button>
          </section>
        )}
        {streamingReply && (
          <article className="chat-message assistant streaming">
            <span>Goalbar</span>
            <p>{streamingReply}</p>
          </article>
        )}
        {chatPending && !streamingReply && (
          <div className="chat-thinking">
            <LoaderCircle size={13} /> {provider} is thinking…
          </div>
        )}

        {codexToolActivity && (
          <section className="chat-tool-call codex-tool-activity" aria-label="Codex Browser Use activity">
            <div className="chat-tool-call-heading">
              <span>
                <FlaskConical size={14} />
              </span>
              <div>
                <strong>
                  {codexToolActivity.status === "running"
                    ? browserToolLabel(codexToolActivity.tool)
                    : codexToolActivity.status === "completed"
                      ? "Browser Use complete"
                      : "Browser Use paused"}
                </strong>
                <small>Called directly by the persistent Codex chat</small>
              </div>
            </div>
            {codexToolActivity.message && <p className="chat-tool-objective">{codexToolActivity.message}</p>}
          </section>
        )}

        {researchRequest && (
          <section className="chat-tool-call" aria-label="Browser Use activity">
            <div className="chat-tool-call-heading">
              <span>
                <FlaskConical size={14} />
              </span>
              <div>
                <strong>
                  {collect.isPending || progress?.status === "running"
                    ? `Using Browser Use on ${researchTab?.platform?.toUpperCase() ?? "current tab"}`
                    : progress?.status === "completed"
                      ? "Browser Use complete"
                      : progress?.status === "paused" || !researchTab?.platform
                        ? "Browser Use paused"
                        : "Browser Use ready"}
                </strong>
                <small>{researchRequest.reason}</small>
              </div>
            </div>
            <p className="chat-tool-objective">{researchRequest.objective}</p>
            <div className="chat-tool-limits" aria-label="Automatic Browser Use limits">
              <Badge tone="neutral">Read only</Badge>
              <span>Up to {researchRequest.maximumItems} items</span>
              <span>Up to {researchRequest.maximumSteps} steps</span>
            </div>
            {!researchTab?.platform && (
              <small className="tool-hint">
                Open X, LinkedIn, or Reddit and ask again so the tool knows which tab to use.
              </small>
            )}
            {collect.isPending && (
              <div className="chat-thinking">
                <LoaderCircle size={13} /> Observing and navigating the open tab…
              </div>
            )}
            {progress && (
              <div className="run-progress">
                <div>
                  <Badge tone={progress.status === "completed" ? "good" : "warn"}>{progress.status}</Badge>
                  <strong>
                    {progress.itemCount} items · {progress.step} steps
                  </strong>
                </div>
                {progress.summary && <p>{progress.summary}</p>}
              </div>
            )}
          </section>
        )}

        {(trace.length > 0 || findings.length > 0) && (
          <section className="chat-research-results">
            {trace.length > 0 && (
              <div className="research-trace" aria-label="Research action trace">
                {trace.slice(-5).map((item) => (
                  <div key={item.id}>
                    <span>{item.step + 1}</span>
                    <strong>{item.action}</strong>
                    <p>{item.message}</p>
                  </div>
                ))}
              </div>
            )}
            {findings.map((finding) => (
              <article className={`research-finding ${finding.status}`} key={finding.id}>
                <div>
                  <Badge tone={finding.status === "accepted" ? "good" : "neutral"}>
                    {finding.category.replace("_", " ")}
                  </Badge>
                  <small>{Math.round(finding.confidence * 100)}% confidence</small>
                </div>
                <strong>{finding.summary}</strong>
                <blockquote>{finding.evidenceExcerpt}</blockquote>
                {finding.status === "proposed" && (
                  <div className="finding-actions">
                    <Button
                      size="small"
                      disabled={review.isPending}
                      onClick={() => review.mutate({ findingId: finding.id, status: "accepted" })}
                    >
                      <CheckCircle2 size={13} /> Add to ICP memory
                    </Button>
                    <Button
                      variant="secondary"
                      size="small"
                      disabled={review.isPending}
                      onClick={() => review.mutate({ findingId: finding.id, status: "rejected" })}
                    >
                      <XCircle size={13} /> Reject
                    </Button>
                  </div>
                )}
              </article>
            ))}
          </section>
        )}
      </div>

      {error && (
        <div className="inline-error">
          <strong>Chat needs attention</strong>
          <span>{error.message}</span>
        </div>
      )}

      <div className="founder-chat-addons" aria-label="Chat add-ons">
        <button
          className="active"
          onClick={() => {
            if (!composer) setComposer("Research this page for ICP pains, goals, and exact customer language")
          }}
        >
          <FlaskConical size={13} /> Browser Use
          <span>chat callable</span>
        </button>
        <button onClick={() => setHistoryOpen((value) => !value)}>
          <Archive size={13} /> History
        </button>
      </div>

      {historyOpen && <HistoryImportPanel />}

      <form
        className="founder-chat-composer"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <Textarea
          aria-label="Chat message"
          id="founder-chat-message"
          name="founderChatMessage"
          rows={3}
          value={composer}
          onChange={(event) => setComposer(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault()
              submit()
            }
          }}
          placeholder="Ask about your ICP, content, or the visible page…"
        />
        <Button
          size="icon"
          type={provider === "codex" && selectedCodexChatRunning ? "button" : "submit"}
          aria-label={
            provider === "codex" && selectedCodexChatRunning ? "Stop Codex response" : "Send message"
          }
          disabled={
            provider === "codex" && selectedCodexChatRunning
              ? interruptCodexChat.isPending
              : !composer.trim() || chat.isPending || collect.isPending
          }
          onClick={
            provider === "codex" && selectedCodexChatRunning
              ? () => {
                  interruptCodexChat.mutate(activeChatId)
                }
              : undefined
          }
        >
          {selectedCodexChatRunning ? (
            <Square size={13} />
          ) : chat.isPending || collect.isPending ? (
            <LoaderCircle size={15} />
          ) : (
            <Send size={15} />
          )}
        </Button>
      </form>
      <p className="founder-chat-footnote">
        <Sparkles size={11} /> Codex keeps every chat alive. Each one can use the open browser tab; typing,
        publishing, and sending always require you.
      </p>
    </section>
  )
}
