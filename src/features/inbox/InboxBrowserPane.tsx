import { ArrowLeft, ArrowRight, ExternalLink, Globe2, LockKeyhole, RefreshCw } from "lucide-react"
import { useEffect, useRef, useState } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { useBrowserSurface } from "@/features/browser/useBrowserSurface"
import type { Conversation } from "@/schemas/inbox"

type InboxBrowserPaneProps = {
  conversation: Conversation
  onOpenExternally: (url: string) => void
  obscured?: boolean
  targetUrl?: string
  view?: "thread" | "profile"
}

const linkedInInboxUrl = "https://www.linkedin.com/messaging/"

function conversationTargetUrl(conversation: Conversation, targetUrl?: string): string | null {
  if (targetUrl) return targetUrl
  const remoteUrl = conversation.remoteUrl ?? null
  if (!remoteUrl || conversation.platform !== "linkedin") return remoteUrl

  try {
    const parsed = new URL(remoteUrl)
    const segments = parsed.pathname.split("/").filter(Boolean)
    const hasRecoverablePlaceholder =
      segments.length === 4 &&
      segments[0]?.toLocaleLowerCase() === "messaging" &&
      segments[1]?.toLocaleLowerCase() === "thread" &&
      segments[2]?.toLocaleLowerCase() !== "undefined" &&
      segments[3]?.toLocaleLowerCase() === "undefined"
    if (hasRecoverablePlaceholder) {
      parsed.pathname = `/${segments.slice(0, 3).join("/")}/`
      return parsed.toString()
    }
    return segments.some((segment) => segment.toLocaleLowerCase() === "undefined")
      ? linkedInInboxUrl
      : remoteUrl
  } catch {
    return linkedInInboxUrl
  }
}

export function InboxBrowserPane({
  conversation,
  onOpenExternally,
  obscured = false,
  targetUrl,
  view = "thread",
}: InboxBrowserPaneProps) {
  const {
    surfaceRef,
    activeTab,
    error,
    isNative,
    openUrlInPlatform,
    setBrowserViewsObscured,
    back,
    forward,
    reload,
  } = useBrowserSurface()
  const openedTarget = useRef<string | null>(null)
  const [opening, setOpening] = useState(true)
  const [openError, setOpenError] = useState<string | null>(null)
  const remoteUrl = conversationTargetUrl(conversation, targetUrl)
  const targetKey = remoteUrl ? `${conversation.id}:${view}:${remoteUrl}` : null

  useEffect(() => {
    void setBrowserViewsObscured(obscured).catch((reason: unknown) => {
      setOpenError(reason instanceof Error ? reason.message : String(reason))
    })
  }, [obscured, setBrowserViewsObscured])

  useEffect(() => {
    if (!remoteUrl || !targetKey || openedTarget.current === targetKey) return
    openedTarget.current = targetKey
    setOpening(true)
    setOpenError(null)
    void openUrlInPlatform(remoteUrl, conversation.platform)
      .then((tab) => {
        if (!tab) throw new Error("The browser pane is not ready yet. Select the conversation again.")
      })
      .catch((reason: unknown) => {
        openedTarget.current = null
        setOpenError(reason instanceof Error ? reason.message : String(reason))
      })
      .finally(() => setOpening(false))
  }, [conversation, openUrlInPlatform, remoteUrl, targetKey, view])

  return (
    <section className="panel inbox-browser-panel" aria-label={`Live ${conversation.platform} ${view}`}>
      <div className="inbox-browser-heading">
        <div>
          <p className="eyebrow">Live platform {view}</p>
          <h2>{conversation.displayName}</h2>
        </div>
        <Badge tone="good">
          <LockKeyhole size={12} /> Local session
        </Badge>
      </div>

      <div className="inbox-browser-controls">
        <Button variant="ghost" size="icon" aria-label="Browser back" onClick={() => void back()}>
          <ArrowLeft size={15} />
        </Button>
        <Button variant="ghost" size="icon" aria-label="Browser forward" onClick={() => void forward()}>
          <ArrowRight size={15} />
        </Button>
        <Button variant="ghost" size="icon" aria-label="Reload thread" onClick={() => void reload()}>
          <RefreshCw size={14} />
        </Button>
        <div className="inbox-browser-address" title={activeTab?.currentUrl ?? remoteUrl ?? undefined}>
          <LockKeyhole size={12} />
          <span>{activeTab?.currentUrl ?? remoteUrl}</span>
        </div>
        {remoteUrl && (
          <Button
            variant="ghost"
            size="icon"
            aria-label="Open thread in external browser"
            onClick={() => onOpenExternally(remoteUrl)}
          >
            <ExternalLink size={15} />
          </Button>
        )}
      </div>

      <div className="inbox-browser-surface" ref={surfaceRef}>
        {!isNative && (
          <div className="browser-preview-placeholder">
            <Globe2 size={32} />
            <h2>Live browser preview</h2>
            <p>The signed-in platform thread appears here in the Goalbar desktop app.</p>
          </div>
        )}
        {opening && <div className="inbox-browser-state">Opening the real thread…</div>}
        {(openError ?? error) && (
          <div className="inbox-browser-state inbox-browser-state-error" role="alert">
            <strong>Thread could not open</strong>
            <span>{openError ?? error}</span>
          </div>
        )}
      </div>
    </section>
  )
}
