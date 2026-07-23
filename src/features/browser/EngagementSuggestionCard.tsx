import { ArrowUpRight, Check, MessageCircle, RefreshCw, ShieldCheck, ThumbsUp } from "lucide-react"
import { useState } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import type { EngagementSuggestion } from "@/schemas/agent"
import type { BrowserReplyPreparation } from "@/schemas/browser"

type EngagementSuggestionCardProps = {
  suggestion: EngagementSuggestion
  rewritePending: boolean
  onOpen: (url: string) => void
  onPrepare: (url: string, reply: string) => Promise<BrowserReplyPreparation>
  onRewrite: (suggestion: EngagementSuggestion) => void
}

function platformLabel(url: string) {
  try {
    const host = new URL(url).hostname.replace(/^www\./, "")
    if (host === "x.com" || host === "twitter.com") return "X"
    if (host.endsWith("linkedin.com")) return "LinkedIn"
    if (host.endsWith("reddit.com")) return "Reddit"
    return host
  } catch {
    return "Post"
  }
}

export function EngagementSuggestionCard({
  suggestion,
  rewritePending,
  onOpen,
  onPrepare,
  onRewrite,
}: EngagementSuggestionCardProps) {
  const [reviewing, setReviewing] = useState(false)
  const [draft, setDraft] = useState(suggestion.reply)
  const [preparation, setPreparation] = useState<{
    phase: "idle" | "preparing" | "prepared" | "failed"
    status?: BrowserReplyPreparation["status"]
    savedLocally?: boolean
  }>({ phase: "idle" })

  const exactReply = draft.trim()
  const openPost = () => onOpen(suggestion.url)
  const prepareInBrowser = async () => {
    if (!exactReply || preparation.phase === "preparing") return
    setPreparation({ phase: "preparing" })
    try {
      const result = await onPrepare(suggestion.url, exactReply)
      setPreparation({
        phase: result.status === "prepared" ? "prepared" : "failed",
        status: result.status,
        savedLocally: Boolean(result.savedReply),
      })
    } catch {
      setPreparation({ phase: "failed" })
    }
  }

  const statusMessage =
    preparation.phase === "prepared"
      ? preparation.savedLocally
        ? "Ready in the browser and saved locally with this exact text. It is not marked as posted; review it there, then click Comment or Reply yourself."
        : "Ready in the browser — this exact reply is filled in but not posted. Review it there, then click Comment or Reply yourself."
      : preparation.status === "login_required"
        ? "Sign in on the open platform, then try putting this reply in the browser again."
        : preparation.status === "verification_required"
          ? "Finish the platform’s verification in the browser, then try again."
          : preparation.status === "unsupported_page"
            ? "Goalbar can only fill a reply on a supported X, LinkedIn, or Reddit post in the desktop app."
            : preparation.status === "composer_not_found"
              ? "Goalbar couldn’t find the post’s reply box. Open the post fully, then try again."
              : preparation.phase === "failed"
                ? "Goalbar couldn’t prepare the reply in the browser. Open the post and try again."
                : reviewing
                  ? "Review the exact text above. “Put in browser” fills the site’s reply box but never submits it."
                  : "You stay in control of the final like and reply."

  return (
    <article className="engagement-card" aria-label="Recommended engagement">
      <div className="engagement-card-kicker">
        <Badge tone="good">Recommended next move</Badge>
        <small>{platformLabel(suggestion.url)}</small>
      </div>

      <button className="engagement-post" type="button" onClick={openPost}>
        <span className="engagement-platform-mark">
          <ThumbsUp size={14} />
        </span>
        <span>
          <strong>{suggestion.title}</strong>
          <small>Open the post beside this chat</small>
        </span>
        <ArrowUpRight size={14} />
      </button>

      <div className="engagement-reason">
        <strong>Why this one</strong>
        <p>{suggestion.reason}</p>
      </div>

      <div className={`engagement-reply${reviewing ? " reviewing" : ""}`}>
        <div>
          <strong>{reviewing ? "Review your exact reply" : "Suggested reply"}</strong>
          {reviewing && (
            <Badge tone="neutral">
              <ShieldCheck size={11} /> You approve the final text
            </Badge>
          )}
        </div>
        {reviewing ? (
          <>
            <Textarea
              aria-label="Exact reply"
              rows={5}
              value={draft}
              onChange={(event) => {
                setDraft(event.target.value)
                setPreparation({ phase: "idle" })
              }}
            />
            <small className="engagement-character-count">{exactReply.length} characters</small>
          </>
        ) : (
          <blockquote>{draft}</blockquote>
        )}
      </div>

      <div className="engagement-actions">
        {reviewing ? (
          <>
            <Button
              size="small"
              type="button"
              disabled={!exactReply || preparation.phase === "preparing"}
              onClick={() => void prepareInBrowser()}
            >
              {preparation.phase === "prepared" ? <Check size={13} /> : <MessageCircle size={13} />}
              {preparation.phase === "preparing"
                ? "Preparing…"
                : preparation.phase === "prepared"
                  ? "Ready in browser"
                  : "Put in browser"}
            </Button>
            <Button
              variant="secondary"
              size="small"
              type="button"
              disabled={!exactReply || rewritePending}
              onClick={() => onRewrite({ ...suggestion, reply: exactReply })}
            >
              <RefreshCw size={13} /> {rewritePending ? "Rewriting…" : "Rewrite"}
            </Button>
            <Button variant="ghost" size="small" type="button" onClick={() => setReviewing(false)}>
              Cancel
            </Button>
          </>
        ) : (
          <>
            <Button size="small" type="button" onClick={() => setReviewing(true)}>
              <MessageCircle size={13} /> Comment
            </Button>
            <Button
              variant="secondary"
              size="small"
              type="button"
              disabled={rewritePending}
              onClick={() => onRewrite({ ...suggestion, reply: exactReply })}
            >
              <RefreshCw size={13} /> {rewritePending ? "Rewriting…" : "Rewrite"}
            </Button>
            <Button variant="ghost" size="small" type="button" onClick={openPost}>
              <ArrowUpRight size={13} /> Open
            </Button>
          </>
        )}
      </div>

      <p className={`engagement-status ${preparation.phase}`} aria-live="polite">
        {statusMessage}
      </p>
    </article>
  )
}
