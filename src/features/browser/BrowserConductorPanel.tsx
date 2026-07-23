import { useMutation } from "@tanstack/react-query"
import { listen } from "@tauri-apps/api/event"
import { Check, Copy, Eye, MousePointer2, Play, ShieldCheck } from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { z } from "zod"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  browserCaptureInputSchema,
  browserCapturePreviewSchema,
  browserRunProgressSchema,
  cancelBrowserCollectionInputSchema,
  startBrowserCollectionInputSchema,
  type BrowserCaptureInput,
  type BrowserRunProgress,
  type BrowserTab,
} from "@/schemas/browser"
import { historyImportResultSchema } from "@/schemas/history"

type BrowserConductorPanelProps = {
  activeTab: BrowserTab | null
}

export function BrowserConductorPanel({ activeTab }: BrowserConductorPanelProps) {
  const [provider, setProvider] = useState<"codex" | "claude">("codex")
  const [ownership, setOwnership] = useState<"own" | "reference">("reference")
  const [objective, setObjective] = useState("Collect a small, relevant sample for ICP research")
  const [maximumItems, setMaximumItems] = useState(50)
  const [maximumSteps, setMaximumSteps] = useState(25)
  const [boundsConfirmed, setBoundsConfirmed] = useState(false)
  const [draft, setDraft] = useState("")
  const [copied, setCopied] = useState(false)
  const [progress, setProgress] = useState<BrowserRunProgress | null>(null)

  useEffect(() => {
    if (!isTauriRuntime()) return
    const unlisten = listen<unknown>("browser://run-progress", (event) => {
      setProgress(browserRunProgressSchema.parse(event.payload))
    })
    return () => {
      void unlisten.then((dispose) => dispose())
    }
  }, [])

  const captureInput = (mode: BrowserCaptureInput["mode"]): BrowserCaptureInput => {
    if (!activeTab) throw new Error("Open a browser tab first")
    return { tabId: activeTab.id, mode, ownership }
  }

  const preview = useMutation({
    mutationFn: async (mode: BrowserCaptureInput["mode"]) => {
      const input = captureInput(mode)
      if (!isTauriRuntime())
        return browserCapturePreviewSchema.parse({
          observation: {
            schemaVersion: 1,
            tabId: input.tabId,
            url: activeTab?.currentUrl ?? "https://x.com/home",
            title: activeTab?.title ?? "Preview",
            platform: activeTab?.platform ?? "x",
            pageKind: "feed",
            viewport: { width: 900, height: 700, scrollY: 0 },
            visibleBlocks: [
              {
                key: "preview",
                role: "article",
                text: "A selected founder insight appears here before anything is stored.",
                links: [],
              },
            ],
            capturedItemKeys: [],
          },
          selectedText: mode === "selection" ? "A selected founder insight." : null,
          normalizedItemCount: 1,
          policyState: "explicit_capture",
        })
      return invokeValidated(
        "preview_browser_capture",
        { input },
        browserCaptureInputSchema,
        browserCapturePreviewSchema,
      )
    },
    onSuccess: (value) => setDraft(value.selectedText ?? value.observation.visibleBlocks[0]?.text ?? ""),
  })

  const commit = useMutation({
    mutationFn: async (mode: BrowserCaptureInput["mode"]) => {
      const input = captureInput(mode)
      if (!isTauriRuntime())
        return historyImportResultSchema.parse({
          sourceId: crypto.randomUUID(),
          runId: crypto.randomUUID(),
          platform: activeTab?.platform ?? "x",
          imported: 1,
          skipped: 0,
          warningCount: 0,
          duplicateSource: false,
        })
      return invokeValidated(
        "commit_browser_capture",
        { input },
        browserCaptureInputSchema,
        historyImportResultSchema,
      )
    },
  })

  const collect = useMutation({
    mutationFn: async () => {
      if (!activeTab) throw new Error("Open a browser tab first")
      const input = {
        tabId: activeTab.id,
        objective,
        limits: { maximumItems, maximumSteps },
        ownership,
        provider,
      }
      if (!isTauriRuntime())
        throw new Error(
          "Automated website collection is policy-gated. Use explicit capture or import an official archive.",
        )
      return invokeValidated(
        "start_browser_collection",
        { input },
        startBrowserCollectionInputSchema,
        browserRunProgressSchema,
      )
    },
    onSuccess: (value) => setProgress(value),
  })
  const cancel = useMutation({
    mutationFn: async () => {
      if (!progress) return false
      const input = { runId: progress.runId }
      return invokeValidated(
        "cancel_browser_collection",
        { input },
        cancelBrowserCollectionInputSchema,
        z.boolean(),
      )
    },
  })

  const error = preview.error ?? commit.error ?? collect.error ?? cancel.error
  const captureMode = useMemo(
    () => (preview.variables === "selection" ? "selection" : "visible"),
    [preview.variables],
  )

  return (
    <div className="conductor-stack">
      <section className="conductor-section">
        <div className="conductor-heading">
          <span>
            <ShieldCheck size={16} />
          </span>
          <div>
            <h2>Research Agent</h2>
            <p>Uses your local Codex or Claude session to reason over approved evidence.</p>
          </div>
        </div>
        <div className="segmented" aria-label="Reasoning provider">
          {(["codex", "claude"] as const).map((value) => (
            <button
              className={provider === value ? "active" : ""}
              key={value}
              onClick={() => setProvider(value)}
            >
              {value}
            </button>
          ))}
        </div>
        <label className="field">
          <span>Evidence ownership</span>
          <select
            value={ownership}
            onChange={(event) => setOwnership(event.target.value as typeof ownership)}
          >
            <option value="reference">ICP/reference account</option>
            <option value="own">My account</option>
          </select>
        </label>
        <div className="capture-actions">
          <Button
            variant="secondary"
            size="small"
            disabled={!activeTab || preview.isPending}
            onClick={() => preview.mutate("visible")}
          >
            <Eye size={14} /> Preview visible
          </Button>
          <Button
            variant="secondary"
            size="small"
            disabled={!activeTab || preview.isPending}
            onClick={() => preview.mutate("selection")}
          >
            <MousePointer2 size={14} /> Preview selection
          </Button>
        </div>
        {preview.data && (
          <div className="capture-preview">
            <div>
              <Badge tone="good">{preview.data.normalizedItemCount} normalized</Badge>
              <small>{preview.data.observation.url}</small>
            </div>
            <p>{preview.data.selectedText ?? preview.data.observation.visibleBlocks[0]?.text}</p>
            <Button size="small" onClick={() => commit.mutate(captureMode)} disabled={commit.isPending}>
              {commit.isPending ? "Saving…" : "Save this capture"}
            </Button>
          </div>
        )}
        {commit.data && (
          <p className="success-note">
            <Check size={14} /> Saved {commit.data.imported} item{commit.data.imported === 1 ? "" : "s"}{" "}
            locally.
          </p>
        )}
      </section>

      <section className="conductor-section">
        <div className="section-kicker">
          <Play size={14} /> Bounded collection
        </div>
        <label className="field">
          <span>Objective</span>
          <Textarea
            rows={3}
            value={objective}
            onChange={(event) => {
              setObjective(event.target.value)
              setBoundsConfirmed(false)
            }}
          />
        </label>
        <div className="field-grid two">
          <label className="field">
            <span>Max items</span>
            <Input
              type="number"
              min={1}
              max={500}
              value={maximumItems}
              onChange={(event) => {
                setMaximumItems(Number(event.target.value))
                setBoundsConfirmed(false)
              }}
            />
          </label>
          <label className="field">
            <span>Max steps</span>
            <Input
              type="number"
              min={1}
              max={100}
              value={maximumSteps}
              onChange={(event) => {
                setMaximumSteps(Number(event.target.value))
                setBoundsConfirmed(false)
              }}
            />
          </label>
        </div>
        <label className="bounds-confirmation">
          <input
            type="checkbox"
            checked={boundsConfirmed}
            onChange={(event) => setBoundsConfirmed(event.target.checked)}
          />
          <span>I confirm this objective and these hard limits.</span>
        </label>
        <Button
          disabled={!activeTab || !boundsConfirmed || collect.isPending}
          onClick={() => collect.mutate()}
        >
          {collect.isPending ? "Collecting…" : "Check policy and start"}
        </Button>
        {progress && (
          <div className="run-progress">
            <div>
              <Badge tone={progress.status === "completed" ? "good" : "warn"}>{progress.status}</Badge>
              <strong>
                {progress.itemCount} items · step {progress.step}
              </strong>
            </div>
            {progress.summary && <p>{progress.summary}</p>}
            {progress.status === "running" && (
              <Button variant="secondary" size="small" onClick={() => cancel.mutate()}>
                Cancel run
              </Button>
            )}
          </div>
        )}
        <p className="fine-print">
          Collection pauses on login, verification, rate limits, host changes, or uncertainty.
        </p>
      </section>

      {draft && (
        <section className="conductor-section">
          <div className="section-kicker">
            <Copy size={14} /> Working note
          </div>
          <Textarea value={draft} rows={5} onChange={(event) => setDraft(event.target.value)} />
          <Button
            variant="secondary"
            size="small"
            onClick={() => {
              void navigator.clipboard.writeText(draft)
              setCopied(true)
            }}
          >
            {copied ? <Check size={14} /> : <Copy size={14} />}
            {copied ? "Copied exact text" : "Copy exact text"}
          </Button>
          <p className="fine-print">
            Copying never means published. You still review and click Publish or Send.
          </p>
        </section>
      )}

      {error && (
        <div className="inline-error">
          <strong>Capture needs attention</strong>
          <span>{error.message}</span>
        </div>
      )}
    </div>
  )
}
