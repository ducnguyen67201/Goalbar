import { useMutation, useQuery } from "@tanstack/react-query"
import { ArrowUpRight, Check, Inbox, Send, Sparkles } from "lucide-react"
import { useState } from "react"
import { z } from "zod"

import { CapabilityBadge } from "@/components/CapabilityBadge"
import { EmptyState } from "@/components/EmptyState"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { titleCase } from "@/lib/utils"
import { approvalSchema } from "@/schemas/content"
import {
  conversationsSchema,
  remoteMessageSchema,
  replyOptionsSchema,
  type Conversation,
} from "@/schemas/inbox"

const draftInputSchema = z.object({
  provider: z.enum(["codex", "claude"]),
  conversationId: z.string().uuid(),
})
const approveInputSchema = z.object({ conversationId: z.string().uuid(), body: z.string().min(1) })
const sendInputSchema = approveInputSchema.extend({
  approvalId: z.string().uuid(),
  recipientId: z.string().optional(),
})

export function InboxPage() {
  const [selected, setSelected] = useState<Conversation | null>(null)
  const [body, setBody] = useState("")
  const [recipientId, setRecipientId] = useState("")
  const [approvalId, setApprovalId] = useState<string | null>(null)
  const conversations = useQuery({
    queryKey: queryKeys.conversations,
    queryFn: () =>
      isTauriRuntime() ? invokeOutput("list_conversations", {}, conversationsSchema) : Promise.resolve([]),
  })
  const draft = useMutation({
    mutationFn: async () => {
      if (!selected) throw new Error("Select a conversation")
      const input = { provider: "codex" as const, conversationId: selected.id }
      if (!isTauriRuntime()) return { options: ["Thanks for asking—here is what I have learned so far."] }
      return invokeValidated("draft_reply", { input }, draftInputSchema, replyOptionsSchema)
    },
    onSuccess: (value) => setBody(value.options[0] ?? ""),
  })
  const approve = useMutation({
    mutationFn: async () => {
      if (!selected) throw new Error("Select a conversation")
      const input = { conversationId: selected.id, body }
      if (!isTauriRuntime())
        return approvalSchema.parse({
          id: crypto.randomUUID(),
          subjectType: "reply",
          subjectId: selected.id,
          payloadHash: "preview",
          idempotencyKey: crypto.randomUUID(),
          approvedAt: new Date().toISOString(),
        })
      return invokeValidated("approve_reply", { input }, approveInputSchema, approvalSchema)
    },
    onSuccess: (approval) => setApprovalId(approval.id),
  })
  const send = useMutation({
    mutationFn: async () => {
      if (!selected || !approvalId) throw new Error("Approve this exact text first")
      const input = { conversationId: selected.id, approvalId, body, recipientId: recipientId || undefined }
      if (!isTauriRuntime())
        return remoteMessageSchema.parse({ platform: selected.platform, remoteId: crypto.randomUUID(), body })
      return invokeValidated("send_reply", { input }, sendInputSchema, remoteMessageSchema)
    },
    onSuccess: () => {
      setBody("")
      setApprovalId(null)
      setSelected(null)
    },
  })

  if (!conversations.data?.length) {
    return (
      <div className="page-stack">
        <header className="page-header">
          <div>
            <p className="eyebrow">Inbox · relationships over volume</p>
            <h1>Conversations worth continuing.</h1>
          </div>
        </header>
        <EmptyState
          eyebrow="Nothing waiting"
          title="Your unified inbox is quiet."
          body="Supported comments and direct messages will appear after you connect accounts and run a sync. LinkedIn member DMs remain an Open in LinkedIn action."
        />
      </div>
    )
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Inbox · relationships over volume</p>
          <h1>Conversations worth continuing.</h1>
        </div>
      </header>
      <div className="inbox-layout">
        <div className="conversation-list">
          {conversations.data.map((conversation) => (
            <button
              className="conversation-row conversation-button"
              key={conversation.id}
              onClick={() => {
                setSelected(conversation)
                setBody("")
                setApprovalId(null)
              }}
            >
              <span className="avatar">
                <Inbox size={17} />
              </span>
              <div>
                <div className="conversation-meta">
                  <strong>{conversation.displayName}</strong>
                  <span>{titleCase(conversation.platform)}</span>
                </div>
                <p>{conversation.preview}</p>
              </div>
              <CapabilityBadge state={conversation.replyCapability} />
            </button>
          ))}
        </div>
        <section className="panel reply-panel">
          {!selected ? (
            <EmptyState
              eyebrow="Choose a thread"
              title="Review before you draft."
              body="The agent receives only the selected conversation and approved founder memory."
            />
          ) : (
            <>
              <div className="panel-heading">
                <span className="panel-icon">
                  <Sparkles size={17} />
                </span>
                <div>
                  <h2>Reply to {selected.displayName}</h2>
                  <p>
                    {titleCase(selected.platform)} · {titleCase(selected.kind)}
                  </p>
                </div>
                {selected.remoteUrl && (
                  <Button variant="ghost" size="icon" aria-label="Open on platform">
                    <ArrowUpRight size={16} />
                  </Button>
                )}
              </div>
              {selected.kind === "direct_message" && (
                <label className="field">
                  <span>Recipient platform ID</span>
                  <Input value={recipientId} onChange={(event) => setRecipientId(event.target.value)} />
                </label>
              )}
              <Textarea
                rows={10}
                value={body}
                onChange={(event) => {
                  setBody(event.target.value)
                  setApprovalId(null)
                }}
                placeholder="Draft a thoughtful reply…"
              />
              <div className="reply-actions">
                <Button variant="secondary" onClick={() => draft.mutate()} disabled={draft.isPending}>
                  {draft.isPending ? "Drafting…" : "Draft with Codex"}
                </Button>
                {approvalId ? (
                  <Button onClick={() => send.mutate()} disabled={send.isPending}>
                    {send.isPending ? (
                      "Sending…"
                    ) : (
                      <>
                        <Send size={14} /> Send approved text
                      </>
                    )}
                  </Button>
                ) : (
                  <Button onClick={() => approve.mutate()} disabled={!body || approve.isPending}>
                    <Check size={14} /> Approve exact text
                  </Button>
                )}
              </div>
              {(draft.error || approve.error || send.error) && (
                <div className="inline-error">
                  <strong>Action could not finish</strong>
                  <span>{draft.error?.message ?? approve.error?.message ?? send.error?.message}</span>
                </div>
              )}
            </>
          )}
        </section>
      </div>
    </div>
  )
}
