import { Globe2, LockKeyhole } from "lucide-react"
import { useEffect, useState } from "react"

import { PaneDivider } from "@/components/PaneDivider"
import { Badge } from "@/components/ui/badge"
import { BrowserConductorPanel } from "@/features/browser/BrowserConductorPanel"
import { BrowserStartPage } from "@/features/browser/BrowserStartPage"
import { BrowserToolbar } from "@/features/browser/BrowserToolbar"
import { HistoryImportPanel } from "@/features/browser/HistoryImportPanel"
import { useBrowserSurface } from "@/features/browser/useBrowserSurface"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { browserPanelWidthInputSchema, browserPanelWidthSchema } from "@/schemas/browser"

export function BrowserPage() {
  const {
    surfaceRef,
    tabs,
    activeTab,
    startPageOpen,
    error,
    isNative,
    newWindowUrl,
    dismissNewWindow,
    openNewWindow,
    createTab,
    openStartPage,
    closeStartPage,
    activate,
    navigate,
    close,
    back,
    forward,
    reload,
  } = useBrowserSurface()
  const [panelWidth, setPanelWidth] = useState(340)

  useEffect(() => {
    if (!isTauriRuntime()) return
    const timeout = window.setTimeout(() => {
      const input = { width: panelWidth }
      void invokeValidated(
        "set_browser_panel_width",
        { input },
        browserPanelWidthInputSchema,
        browserPanelWidthSchema,
      ).catch(() => undefined)
    }, 150)
    return () => window.clearTimeout(timeout)
  }, [panelWidth])

  useEffect(() => {
    if (!isTauriRuntime()) return
    void invokeOutput("get_browser_panel_width", {}, browserPanelWidthSchema.nullable())
      .then((width) => {
        if (width !== null) setPanelWidth(width)
      })
      .catch(() => undefined)
  }, [])

  return (
    <div className="browser-workspace" style={{ gridTemplateColumns: `${panelWidth}px 8px minmax(0, 1fr)` }}>
      <aside className="browser-conductor-pane">
        <div className="browser-pane-header">
          <div>
            <p className="eyebrow">Browser · local session</p>
            <h1>Research without switching.</h1>
          </div>
          <Badge tone="good">
            <LockKeyhole size={12} /> Isolated
          </Badge>
        </div>
        {newWindowUrl && (
          <div className="browser-new-window-request">
            <div>
              <strong>Page requested a new tab</strong>
              <span>{newWindowUrl}</span>
            </div>
            <button onClick={dismissNewWindow}>Dismiss</button>
            <button onClick={() => void openNewWindow()}>Open visibly</button>
          </div>
        )}
        <BrowserConductorPanel activeTab={activeTab} />
        <HistoryImportPanel />
      </aside>
      <PaneDivider
        label="Resize browser controls"
        onMove={(delta) => setPanelWidth((value) => Math.max(280, Math.min(480, value + delta)))}
      />
      <section className="browser-pane" aria-label="Integrated browser">
        <BrowserToolbar
          key={`${startPageOpen ? "start" : (activeTab?.id ?? "empty")}:${activeTab?.currentUrl ?? ""}`}
          tabs={tabs}
          activeTab={activeTab}
          startPageOpen={startPageOpen}
          onCreate={openStartPage}
          onCloseStartPage={closeStartPage}
          onActivate={activate}
          onClose={close}
          onNavigate={navigate}
          onBack={back}
          onForward={forward}
          onReload={reload}
        />
        <div className="browser-surface-slot" ref={surfaceRef}>
          {startPageOpen && <BrowserStartPage onOpen={createTab} />}
          {!startPageOpen && !isNative && (
            <div className="browser-preview-placeholder">
              <Globe2 size={34} />
              <h2>Integrated browser preview</h2>
              <p>
                In the desktop app this surface is a native child webview. Sign-ins remain in the local
                website profile and never enter Tagline memory.
              </p>
              <small>{activeTab?.currentUrl}</small>
            </div>
          )}
        </div>
        {error && (
          <div className="browser-engine-error">
            <strong>Browser engine unavailable</strong>
            <span>{error}</span>
          </div>
        )}
      </section>
    </div>
  )
}
