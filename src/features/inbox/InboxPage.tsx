import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { ArrowUpRight, Check, Copy, Inbox, MailCheck, RefreshCw, Send, Sparkles } from "lucide-react"
import { useMemo, useState } from "react"
import { z } from "zod"

import { CapabilityBadge } from "@/components/CapabilityBadge"
import { EmptyState } from "@/components/EmptyState"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { InboxBrowserPane } from "@/features/inbox/InboxBrowserPane"
import { relativeDate } from "@/lib/dates"
import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { titleCase } from "@/lib/utils"
import { approvalSchema } from "@/schemas/content"
import {
  browserInboxScanInputSchema,
  browserInboxScanResultSchema,
  conversationsSchema,
  emailNotificationSyncResultSchema,
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
const conversationInputSchema = z.object({ conversationId: z.string().uuid() })
const openUrlSchema = z.url()
type InboxFilter = "all" | "new" | Conversation["platform"]

export function InboxPage() {
  const queryClient = useQueryClient()
  const [selected, setSelected] = useState<Conversation | null>(null)
  const [body, setBody] = useState("")
  const [recipientId, setRecipientId] = useState("")
  const [approvalId, setApprovalId] = useState<string | null>(null)
  const [filter, setFilter] = useState<InboxFilter>("all")
  const [copied, setCopied] = useState(false)
  const conversations = useQuery({
    queryKey: queryKeys.conversations,
    queryFn: () =>
      isTauriRuntime() ? invokeOutput("list_conversations", {}, conversationsSchema) : Promise.resolve([]),
  })
  const sync = useMutation({
    mutationFn: () =>
      isTauriRuntime()
        ? invokeOutput("sync_email_notifications", {}, emailNotificationSyncResultSchema)
        : Promise.resolve(
            emailNotificationSyncResultSchema.parse({
              source: "apple_mail",
              scanned: 0,
              imported: 0,
              ignored: 0,
              duplicates: 0,
              platformCounts: { x: 0, reddit: 0, linkedin: 0 },
              lastCheckedAt: new Date().toISOString(),
            }),
          ),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: queryKeys.conversations }),
  })
  const browserScan = useMutation({
    mutationFn: (platform: Conversation["platform"]) => {
      const input = { platform }
      return isTauriRuntime()
        ? invokeValidated(
            "scan_browser_inbox",
            { input },
            browserInboxScanInputSchema,
            browserInboxScanResultSchema,
          )
        : Promise.resolve(
            browserInboxScanResultSchema.parse({
              platform,
              status: "completed",
              scanned: 0,
              imported: 0,
              updated: 0,
              lastScannedAt: new Date().toISOString(),
              message: "Browser preview mode does not scan live platform pages.",
              targetUrl:
                platform === "x"
                  ? "https://x.com/messages"
                  : platform === "reddit"
                    ? "https://www.reddit.com/message/inbox"
                    : "https://www.linkedin.com/messaging/",
            }),
          )
    },
    onSuccess: async (result) => {
      if (result.status === "completed" || result.status === "partial") {
        await queryClient.invalidateQueries({ queryKey: queryKeys.conversations })
      }
    },
  })
  const markRead = useMutation({
    mutationFn: (conversationId: string) => {
      const input = { conversationId }
      return isTauriRuntime()
        ? invokeValidated("mark_conversation_read", { input }, conversationInputSchema, z.boolean())
        : Promise.resolve(true)
    },
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: queryKeys.conversations }),
  })
  const draft = useMutation({
    mutationFn: async () => {
      if (!selected) throw new Error("Select a conversation")
      const input = { provider: "codex" as const, conversationId: selected.id }
      if (!isTauriRuntime()) return { options: ["Thanks for asking—here is what I have learned so far."] }
      return invokeValidated("draft_reply", { input }, draftInputSchema, replyOptionsSchema)
    },
    onSuccess: (value) => {
      setBody(value.options[0] ?? "")
      setCopied(false)
    },
  })
  const approve = useMutation({
    mutationFn: async () => {
      if (!selected) throw new Error("Select a conversation")
      const input = { conversationId: selected.id, body }
      if (!isTauriRuntime())
        return approvalSchema.parse({
          id: crypto.randomUUID(),
          subjectType: selected.kind === "direct_message" ? "direct_message" : "reply",
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
  const openPlatform = useMutation({
    mutationFn: (url: string) =>
      isTauriRuntime()
        ? invokeValidated("open_remote_url", { url }, openUrlSchema, z.void())
        : Promise.resolve(),
  })
  const copyApproved = useMutation({
    mutationFn: async () => {
      if (!approvalId) throw new Error("Approve this exact text first")
      await navigator.clipboard.writeText(body)
    },
    onSuccess: () => setCopied(true),
  })

  const newCount = conversations.data?.filter((conversation) => conversation.unreadCount > 0).length ?? 0
  const visibleConversations = useMemo(() => {
    const rows = conversations.data ?? []
    if (filter === "new") return rows.filter((conversation) => conversation.unreadCount > 0)
    if (filter === "all") return rows
    return rows.filter((conversation) => conversation.platform === filter)
  }, [conversations.data, filter])

  const selectConversation = (conversation: Conversation) => {
    setSelected({ ...conversation, unreadCount: 0 })
    setBody("")
    setApprovalId(null)
    setCopied(false)
    if (conversation.unreadCount > 0) markRead.mutate(conversation.id)
  }
  const localPreview = selected?.source !== "platform_api"

  const actionError =
    conversations.error ??
    sync.error ??
    browserScan.error ??
    markRead.error ??
    draft.error ??
    approve.error ??
    send.error ??
    openPlatform.error ??
    copyApproved.error

  return (
    <div className="page-stack">
      <header className="page-header inbox-page-header">
        <div>
          <p className="eyebrow">Inbox · local browser signals, platform truth</p>
          <h1>Conversations worth continuing.</h1>
        </div>
        <div className="inbox-header-actions" aria-label="Inbox scan actions">
          {(["x", "reddit", "linkedin"] as const).map((platform) => {
            const name = platform === "x" ? "X" : platform === "reddit" ? "Reddit" : "LinkedIn"
            const pending = browserScan.isPending && browserScan.variables === platform
            return (
              <Button
                key={platform}
                variant="secondary"
                aria-label={`Scan ${name} inbox`}
                onClick={() => browserScan.mutate(platform)}
                disabled={browserScan.isPending}
              >
                <RefreshCw size={14} className={pending ? "spin" : undefined} />
                {pending ? `Scanning ${name}…` : `Scan ${name}`}
              </Button>
            )
          })}
          <Button
            variant="ghost"
            aria-label="Check Apple Mail"
            onClick={() => sync.mutate()}
            disabled={sync.isPending}
          >
            {sync.isPending ? <RefreshCw size={14} className="spin" /> : <MailCheck size={14} />}
            {sync.isPending ? "Checking mail…" : "Check mail"}
          </Button>
        </div>
      </header>

      <div className="inbox-toolbar" aria-label="Inbox filters">
        <label className="field inbox-filter">
          <span>Show</span>
          <select
            className="input"
            aria-label="Filter conversations"
            value={filter}
            onChange={(event) => setFilter(event.target.value as InboxFilter)}
          >
            <option value="all">All conversations</option>
            <option value="new">New ({newCount})</option>
            <option value="x">X</option>
            <option value="reddit">Reddit</option>
            <option value="linkedin">LinkedIn</option>
          </select>
        </label>
        <div className="inbox-source-note">
          <strong>Free local connector</strong>
          <span>
            Scans recent conversation rows from signed-in Goalbar browser tabs. Full threads stay on each
            platform.
          </span>
        </div>
        {browserScan.data && (
          <div className="inbox-sync-result" role="status">
            <strong>
              {browserScan.data.status === "completed" || browserScan.data.status === "partial"
                ? `${browserScan.data.scanned} conversations · ${browserScan.data.imported} new · ${browserScan.data.updated} updated`
                : `${titleCase(browserScan.data.platform)} needs attention`}
            </strong>
            <span>{browserScan.data.message}</span>
          </div>
        )}
        {sync.data && (
          <div className="inbox-sync-result" role="status">
            <strong>{sync.data.imported} new</strong>
            <span>
              {sync.data.platformCounts.x} X · {sync.data.platformCounts.reddit} Reddit ·{" "}
              {sync.data.platformCounts.linkedin} LinkedIn
            </span>
          </div>
        )}
      </div>

      {!conversations.isPending && !visibleConversations.length ? (
        <EmptyState
          eyebrow={filter === "all" ? "Nothing waiting" : "No matching notifications"}
          title={filter === "all" ? "Your attention inbox is quiet." : "Try another filter."}
          body={
            filter === "all"
              ? "Open and sign in to a platform in Goalbar Browser, then run its inbox scan."
              : "Goalbar only shows conversations that match the selected platform or new state."
          }
        />
      ) : (
        <div className="inbox-layout">
          <div className="conversation-list">
            {visibleConversations.map((conversation) => (
              <button
                className="conversation-row conversation-button"
                data-unread={conversation.unreadCount > 0}
                data-selected={selected?.id === conversation.id}
                key={conversation.id}
                onClick={() => selectConversation(conversation)}
              >
                <span className="avatar">
                  <Inbox size={17} />
                </span>
                <div>
                  <div className="conversation-meta">
                    <strong>{conversation.displayName}</strong>
                    <span>
                      {titleCase(conversation.platform)} · {relativeDate(conversation.updatedAt)}
                    </span>
                  </div>
                  <p>{conversation.preview}</p>
                </div>
                <div>
                  {conversation.unreadCount > 0 && <span className="unread-dot" aria-label="New" />}
                  {conversation.source === "email_notification" ? (
                    <Badge tone="warn">Email excerpt</Badge>
                  ) : conversation.source === "browser_scan" ? (
                    <Badge tone="good">Browser preview</Badge>
                  ) : (
                    <CapabilityBadge state={conversation.replyCapability} />
                  )}
                </div>
              </button>
            ))}
          </div>
          <div className="inbox-detail-stack">
            {selected?.remoteUrl && (
              <InboxBrowserPane
                conversation={selected}
                onOpenExternally={(url) => openPlatform.mutate(url)}
              />
            )}
            <section className="panel reply-panel">
              {!selected ? (
                <EmptyState
                  eyebrow="Choose a notification"
                  title="Open the real conversation here."
                  body="Select a row to show its signed-in platform thread beside the inbox."
                />
              ) : (
                <>
                  <div className="panel-heading inbox-panel-heading">
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
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label="Open on platform"
                        onClick={() => openPlatform.mutate(selected.remoteUrl!)}
                      >
                        <ArrowUpRight size={16} />
                      </Button>
                    )}
                  </div>
                  {selected.source === "email_notification" && (
                    <div className="inbox-context-warning">
                      <strong>
                        {selected.contentState === "link_only" ? "Link-only notification" : "Email excerpt"}
                      </strong>
                      <span>
                        Open the platform to verify the full conversation. Goalbar will not send
                        automatically.
                      </span>
                    </div>
                  )}
                  {selected.source === "browser_scan" && (
                    <div className="inbox-context-warning">
                      <strong>Browser preview</strong>
                      <span>
                        This is a bounded conversation-list preview. Open the platform to verify the full
                        thread. Goalbar will not send automatically.
                      </span>
                    </div>
                  )}
                  {selected.source === "platform_api" && selected.kind === "direct_message" && (
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
                      setCopied(false)
                    }}
                    placeholder="Draft a thoughtful reply…"
                  />
                  <div className="reply-actions">
                    <Button variant="secondary" onClick={() => draft.mutate()} disabled={draft.isPending}>
                      {draft.isPending ? "Drafting…" : "Draft with Codex"}
                    </Button>
                    {localPreview ? (
                      approvalId ? (
                        <>
                          <Button
                            variant="secondary"
                            onClick={() => copyApproved.mutate()}
                            disabled={copyApproved.isPending}
                          >
                            <Copy size={14} /> {copied ? "Copied" : "Copy approved text"}
                          </Button>
                          {selected.remoteUrl && (
                            <Button onClick={() => openPlatform.mutate(selected.remoteUrl!)}>
                              <ArrowUpRight size={14} /> Open platform
                            </Button>
                          )}
                        </>
                      ) : (
                        <Button onClick={() => approve.mutate()} disabled={!body || approve.isPending}>
                          <Check size={14} /> Approve exact text
                        </Button>
                      )
                    ) : approvalId ? (
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
                </>
              )}
            </section>
          </div>
        </div>
      )}

      {actionError && (
        <div className="inline-error">
          <strong>Inbox action could not finish</strong>
          <span>{actionError.message}</span>
        </div>
      )}
    </div>
  )
}
