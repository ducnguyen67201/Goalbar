import { ArrowLeft, ArrowRight, Plus, RefreshCw, X } from "lucide-react"
import { useState } from "react"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import type { BrowserTab } from "@/schemas/browser"

type BrowserToolbarProps = {
  tabs: BrowserTab[]
  activeTab: BrowserTab | null
  startPageOpen: boolean
  onCreate: () => Promise<void>
  onCloseStartPage: () => Promise<void>
  onActivate: (tabId: string) => Promise<void>
  onClose: (tabId: string) => Promise<void>
  onNavigate: (url: string) => Promise<void>
  onBack: () => Promise<void>
  onForward: () => Promise<void>
  onReload: () => Promise<void>
}

export function BrowserToolbar({
  tabs,
  activeTab,
  startPageOpen,
  onCreate,
  onCloseStartPage,
  onActivate,
  onClose,
  onNavigate,
  onBack,
  onForward,
  onReload,
}: BrowserToolbarProps) {
  const [address, setAddress] = useState(activeTab?.currentUrl ?? "")

  return (
    <div className="browser-toolbar">
      <div className="browser-tabs" role="tablist" aria-label="Browser tabs">
        {tabs.map((tab) => (
          <button
            className={!startPageOpen && tab.active ? "browser-tab active" : "browser-tab"}
            role="tab"
            aria-selected={!startPageOpen && tab.active}
            key={tab.id}
            onClick={() => void onActivate(tab.id)}
          >
            <span>{tab.title || tab.platform || "New tab"}</span>
            <X
              size={12}
              aria-label={`Close ${tab.title || "tab"}`}
              onClick={(event) => {
                event.stopPropagation()
                void onClose(tab.id)
              }}
            />
          </button>
        ))}
        {startPageOpen && (
          <button className="browser-tab active" role="tab" aria-selected="true">
            <span>New tab</span>
            {tabs.length > 0 && (
              <X
                size={12}
                aria-label="Close new tab"
                onClick={(event) => {
                  event.stopPropagation()
                  void onCloseStartPage()
                }}
              />
            )}
          </button>
        )}
        <Button
          variant="ghost"
          size="icon"
          aria-label="New browser tab"
          disabled={startPageOpen || tabs.length >= 5}
          onClick={() => void onCreate()}
        >
          <Plus size={15} />
        </Button>
      </div>
      <form
        className="browser-address-row"
        onSubmit={(event) => {
          event.preventDefault()
          const value = address.startsWith("https://") ? address : `https://${address}`
          void onNavigate(value)
        }}
      >
        <Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Back"
          disabled={!activeTab}
          onClick={() => void onBack()}
        >
          <ArrowLeft size={16} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Forward"
          disabled={!activeTab}
          onClick={() => void onForward()}
        >
          <ArrowRight size={16} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Reload"
          disabled={!activeTab}
          onClick={() => void onReload()}
        >
          <RefreshCw size={15} />
        </Button>
        <Input
          aria-label="Browser address"
          name="browserAddress"
          value={address}
          onChange={(event) => setAddress(event.target.value)}
          placeholder="Enter an X, Reddit, or LinkedIn URL"
        />
      </form>
    </div>
  )
}
