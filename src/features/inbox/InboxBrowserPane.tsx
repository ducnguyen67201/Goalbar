import { ArrowLeft, ArrowRight, ExternalLink, Globe2, LockKeyhole, RefreshCw } from "lucide-react"
import { useEffect, useRef, useState } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { useBrowserSurface } from "@/features/browser/useBrowserSurface"
import type { Conversation } from "@/schemas/inbox"

type InboxBrowserPaneProps = {
  conversation: Conversation
  onOpenExternally: (url: string) => void
}

export function InboxBrowserPane({ conversation, onOpenExternally }: InboxBrowserPaneProps) {
  const { surfaceRef, activeTab, error, isNative, openUrlInPlatform, back, forward, reload } =
    useBrowserSurface()
  const openedConversation = useRef<string | null>(null)
  const [opening, setOpening] = useState(true)
  const [openError, setOpenError] = useState<string | null>(null)
  const remoteUrl = conversation.remoteUrl

  useEffect(() => {
    if (!remoteUrl || openedConversation.current === conversation.id) return
    openedConversation.current = conversation.id
    setOpening(true)
    setOpenError(null)
    void openUrlInPlatform(remoteUrl, conversation.platform)
      .then((tab) => {
        if (!tab) throw new Error("The browser pane is not ready yet. Select the conversation again.")
      })
      .catch((reason: unknown) => {
        openedConversation.current = null
        setOpenError(reason instanceof Error ? reason.message : String(reason))
      })
      .finally(() => setOpening(false))
  }, [conversation.id, conversation.platform, openUrlInPlatform, remoteUrl])

  return (
    <section className="panel inbox-browser-panel" aria-label={`Live ${conversation.platform} thread`}>
      <div className="inbox-browser-heading">
        <div>
          <p className="eyebrow">Live platform thread</p>
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
