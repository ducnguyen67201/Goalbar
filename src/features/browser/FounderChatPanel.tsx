import { useMutation } from "@tanstack/react-query"
import { listen } from "@tauri-apps/api/event"
import { Archive, Bot, CheckCircle2, FlaskConical, LoaderCircle, Send, Sparkles, XCircle } from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { z } from "zod"

import { useBootstrap } from "@/app/bootstrap"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { HistoryImportPanel } from "@/features/browser/HistoryImportPanel"
import { invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  founderChatAgentResultSchema,
  founderChatOutputJsonSchema,
  founderChatTurnSchema,
  runAgentTaskInputSchema,
  type AgentProvider,
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
  type BrowserRunProgress,
  type BrowserTab,
  type StoredResearchFinding,
} from "@/schemas/browser"

type FounderChatPanelProps = {
  activeTab: BrowserTab | null
}

type ChatMessage = {
  id: string
  role: "assistant" | "user" | "tool"
  body: string
}

const initialMessage: ChatMessage = {
  id: "founder-chat-welcome",
  role: "assistant",
  body: "I’m your founder chat. Ask about your ICP, positioning, content, or the page beside us. When browser evidence is needed, I’ll request the Research add-on.",
}

const chatPrompt = `
You are Goalbar's founder chat: a concise strategic partner for a solo founder.
Help with ICP discovery, founder voice, content, conversations, and sustainable growth.
You can request one local tool named research_current_page when the answer requires evidence from the
currently visible X, LinkedIn, or Reddit page. Never claim you read the page unless that tool has run.
When research is useful, return a researchRequest with a focused objective, a short reason, ownership,
and conservative limits. Otherwise researchRequest must be null.
Never publish, send a DM, or imply an external action occurred.
`.trim()

function newMessage(role: ChatMessage["role"], body: string): ChatMessage {
  return { id: crypto.randomUUID(), role, body }
}

function previewFounderChatTurn(message: string): FounderChatTurn {
  const wantsResearch = /\b(research|analy[sz]e|icp|signal|pain|audience|this (page|feed|profile))\b/i.test(
    message,
  )
  return founderChatTurnSchema.parse({
    reply: wantsResearch
      ? "This needs evidence from the visible page. I’ve prepared a bounded Research add-on request for you to review."
      : "I can help shape that. Give me the audience, the outcome you want, and what you already believe to be true.",
    researchRequest: wantsResearch
      ? {
          objective: message.trim(),
          reason: "The answer depends on current audience language and signals from the visible page.",
          ownership: "reference",
          maximumItems: 30,
          maximumSteps: 8,
        }
      : null,
  })
}

export function FounderChatPanel({ activeTab }: FounderChatPanelProps) {
  const bootstrap = useBootstrap()
  const [provider, setProvider] = useState<AgentProvider>("codex")
  const [messages, setMessages] = useState<ChatMessage[]>([initialMessage])
  const [composer, setComposer] = useState("")
  const [researchRequest, setResearchRequest] = useState<FounderChatResearchRequest | null>(null)
  const [boundsConfirmed, setBoundsConfirmed] = useState(false)
  const [progress, setProgress] = useState<BrowserRunProgress | null>(null)
  const [trace, setTrace] = useState<BrowserResearchTrace[]>([])
  const [findings, setFindings] = useState<StoredResearchFinding[]>([])
  const [historyOpen, setHistoryOpen] = useState(false)

  const agentStatuses = useMemo(
    () => new Map(bootstrap.data?.agents.map((status) => [status.provider, status])),
    [bootstrap.data?.agents],
  )

  useEffect(() => {
    if (!isTauriRuntime()) return
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
    return () => {
      void progressListener.then((dispose) => dispose())
      void traceListener.then((dispose) => dispose())
      void findingListener.then((dispose) => dispose())
    }
  }, [])

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

  const chat = useMutation({
    mutationFn: async (message: string) => {
      if (!isTauriRuntime()) return previewFounderChatTurn(message)
      const input = {
        provider,
        taskKind: "founder_chat",
        prompt: chatPrompt,
        context: {
          conversation: [...messages, newMessage("user", message)]
            .slice(-12)
            .map(({ role, body }) => ({ role, body })),
          activePage: activeTab
            ? {
                title: activeTab.title,
                url: activeTab.currentUrl,
                platform: activeTab.platform ?? null,
              }
            : null,
          tools: [
            {
              name: "research_current_page",
              description:
                "Collect a bounded sample from the visible supported platform page after explicit user approval.",
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
    onMutate: (message) => {
      setMessages((current) => [...current, newMessage("user", message)])
      setComposer("")
    },
    onSuccess: (turn) => {
      setMessages((current) => [...current, newMessage("assistant", turn.reply)])
      setResearchRequest(turn.researchRequest)
      setBoundsConfirmed(false)
      setProgress(null)
      setTrace([])
      setFindings([])
    },
  })

  const collect = useMutation({
    mutationFn: async () => {
      if (!activeTab) throw new Error("Open an X, LinkedIn, or Reddit page first")
      if (!researchRequest) throw new Error("The chat has not requested research")
      const input = {
        tabId: activeTab.id,
        objective: researchRequest.objective,
        limits: {
          maximumItems: researchRequest.maximumItems,
          maximumSteps: researchRequest.maximumSteps,
        },
        ownership: researchRequest.ownership,
        provider,
      }
      if (!isTauriRuntime())
        return browserRunProgressSchema.parse({
          runId: crypto.randomUUID(),
          status: "completed",
          step: 1,
          itemCount: 8,
          newItemCount: 8,
          pauseReason: null,
          summary: "Preview research completed with a bounded sample.",
        })
      return invokeValidated(
        "start_browser_collection",
        { input },
        startBrowserCollectionInputSchema,
        browserRunProgressSchema,
      )
    },
    onMutate: () => {
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

  const submit = () => {
    const message = composer.trim()
    if (message && !chat.isPending) chat.mutate(message)
  }
  const error = chat.error ?? collect.error ?? review.error

  return (
    <section className="founder-chat" aria-label="Founder chat">
      <header className="founder-chat-header">
        <div>
          <span className="chat-agent-mark">
            <Bot size={15} />
          </span>
          <div>
            <strong>Founder chat</strong>
            <small>Local {provider} session</small>
          </div>
        </div>
        <div className="segmented compact" aria-label="Chat provider">
          {(["codex", "claude"] as const).map((candidate) => {
            const status = agentStatuses.get(candidate)
            const unavailable = Boolean(status && status.readiness !== "ready")
            return (
              <button
                className={provider === candidate ? "active" : ""}
                disabled={unavailable}
                key={candidate}
                onClick={() => setProvider(candidate)}
              >
                {candidate}
              </button>
            )
          })}
        </div>
      </header>

      <div className="founder-chat-messages" aria-live="polite">
        {messages.map((message) => (
          <article className={`chat-message ${message.role}`} key={message.id}>
            <span>{message.role === "user" ? "You" : message.role === "tool" ? "Research" : "Goalbar"}</span>
            <p>{message.body}</p>
          </article>
        ))}
        {chat.isPending && (
          <div className="chat-thinking">
            <LoaderCircle size={13} /> {provider} is thinking…
          </div>
        )}

        {researchRequest && (
          <section className="chat-tool-call" aria-label="Research add-on request">
            <div className="chat-tool-call-heading">
              <span>
                <FlaskConical size={14} />
              </span>
              <div>
                <strong>Research add-on requested</strong>
                <small>{researchRequest.reason}</small>
              </div>
            </div>
            <label className="field">
              <span>Objective</span>
              <Textarea
                aria-label="Research objective"
                id="chat-research-objective"
                name="chatResearchObjective"
                rows={3}
                value={researchRequest.objective}
                onChange={(event) => {
                  setResearchRequest({ ...researchRequest, objective: event.target.value })
                  setBoundsConfirmed(false)
                }}
              />
            </label>
            <div className="field-grid two">
              <label className="field">
                <span>Max items</span>
                <Input
                  aria-label="Research maximum items"
                  id="chat-research-maximum-items"
                  name="chatResearchMaximumItems"
                  type="number"
                  min={1}
                  max={500}
                  value={researchRequest.maximumItems}
                  onChange={(event) => {
                    setResearchRequest({
                      ...researchRequest,
                      maximumItems: Number(event.target.value),
                    })
                    setBoundsConfirmed(false)
                  }}
                />
              </label>
              <label className="field">
                <span>Max steps</span>
                <Input
                  aria-label="Research maximum steps"
                  id="chat-research-maximum-steps"
                  name="chatResearchMaximumSteps"
                  type="number"
                  min={1}
                  max={100}
                  value={researchRequest.maximumSteps}
                  onChange={(event) => {
                    setResearchRequest({
                      ...researchRequest,
                      maximumSteps: Number(event.target.value),
                    })
                    setBoundsConfirmed(false)
                  }}
                />
              </label>
            </div>
            <label className="bounds-confirmation">
              <input
                name="confirmChatResearchBounds"
                type="checkbox"
                checked={boundsConfirmed}
                onChange={(event) => setBoundsConfirmed(event.target.checked)}
              />
              <span>I approve this objective and these hard limits.</span>
            </label>
            <Button
              size="small"
              disabled={
                !activeTab || !boundsConfirmed || collect.isPending || progress?.status === "completed"
              }
              onClick={() => collect.mutate()}
            >
              {collect.isPending
                ? "Researching…"
                : progress?.status === "completed"
                  ? "Research complete"
                  : "Run approved research"}
            </Button>
            {!activeTab && (
              <small className="tool-hint">Open a supported browser page to enable this tool.</small>
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
          <FlaskConical size={13} /> Research
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
          type="submit"
          aria-label="Send message"
          disabled={!composer.trim() || chat.isPending}
        >
          {chat.isPending ? <LoaderCircle size={15} /> : <Send size={15} />}
        </Button>
      </form>
      <p className="founder-chat-footnote">
        <Sparkles size={11} /> Tools receive bounded evidence only. Publishing and sending always require you.
      </p>
    </section>
  )
}
