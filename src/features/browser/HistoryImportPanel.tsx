import { useMutation, useQueryClient } from "@tanstack/react-query"
import { Archive, CheckCircle2, FileArchive, Upload } from "lucide-react"
import { useState } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  historyImportResultSchema,
  historyPreviewSchema,
  historySelectionInputSchema,
  historySelectionSchema,
  type HistorySelection,
} from "@/schemas/history"

export function HistoryImportPanel() {
  const queryClient = useQueryClient()
  const [selection, setSelection] = useState<HistorySelection | null>(null)

  const choose = useMutation({
    mutationFn: async () => {
      if (!isTauriRuntime())
        return historySelectionSchema.parse({
          selectionId: crypto.randomUUID(),
          displayName: "founder-archive.zip",
          sizeBytes: 428_000,
          container: "zip",
          expiresAt: new Date(Date.now() + 1_800_000).toISOString(),
        })
      return invokeOutput("choose_history_archive", {}, historySelectionSchema.nullable())
    },
    onSuccess: (value) => {
      setSelection(value)
      preview.reset()
      imported.reset()
    },
  })

  const preview = useMutation({
    mutationFn: async () => {
      if (!selection) throw new Error("Choose an archive first")
      const input = { selectionId: selection.selectionId }
      if (!isTauriRuntime())
        return historyPreviewSchema.parse({
          schemaVersion: 1,
          selectionId: selection.selectionId,
          platform: "linkedin",
          parserVersion: "linkedin-archive-v1",
          displayName: selection.displayName,
          accountHandle: null,
          categories: [
            { category: "post", count: 42 },
            { category: "comment", count: 86 },
            { category: "connection", count: 112 },
          ],
          estimatedRecords: 240,
          earliestAt: "2022-01-01T00:00:00Z",
          latestAt: "2026-07-20T00:00:00Z",
          warnings: [],
          unsupportedMembers: ["Ads.csv"],
          sourceFingerprint: "preview-fingerprint",
        })
      return invokeValidated(
        "preview_history_archive",
        { input },
        historySelectionInputSchema,
        historyPreviewSchema,
      )
    },
  })

  const imported = useMutation({
    mutationFn: async () => {
      if (!selection || !preview.data) throw new Error("Preview the archive first")
      const input = { selectionId: selection.selectionId }
      if (!isTauriRuntime())
        return historyImportResultSchema.parse({
          sourceId: crypto.randomUUID(),
          runId: crypto.randomUUID(),
          platform: preview.data.platform,
          imported: preview.data.estimatedRecords,
          skipped: 0,
          warningCount: preview.data.warnings.length,
          duplicateSource: false,
        })
      return invokeValidated(
        "import_history_archive",
        { input },
        historySelectionInputSchema,
        historyImportResultSchema,
      )
    },
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: queryKeys.history }),
  })

  const error = choose.error ?? preview.error ?? imported.error

  return (
    <section className="conductor-section history-import">
      <div className="conductor-heading">
        <span>
          <Archive size={16} />
        </span>
        <div>
          <h2>History import</h2>
          <p>Official archives are the fastest route to complete personal history.</p>
        </div>
      </div>
      <Button variant="secondary" size="small" onClick={() => choose.mutate()} disabled={choose.isPending}>
        <Upload size={14} /> Choose X, LinkedIn, or Reddit archive
      </Button>
      {selection && (
        <div className="archive-selection">
          <FileArchive size={17} />
          <span>
            <strong>{selection.displayName}</strong>
            <small>{formatBytes(selection.sizeBytes)} · path remains in Rust memory only</small>
          </span>
          {!preview.data && (
            <Button size="small" onClick={() => preview.mutate()} disabled={preview.isPending}>
              {preview.isPending ? "Inspecting…" : "Preview"}
            </Button>
          )}
        </div>
      )}
      {preview.data && (
        <div className="archive-preview">
          <div className="archive-preview-head">
            <Badge tone="good">{preview.data.platform}</Badge>
            <strong>{preview.data.estimatedRecords} records found</strong>
          </div>
          <div className="category-grid">
            {preview.data.categories.map((category) => (
              <span key={category.category}>
                {category.category} <strong>{category.count}</strong>
              </span>
            ))}
          </div>
          <small>
            {preview.data.earliestAt?.slice(0, 10) ?? "Unknown start"} →{" "}
            {preview.data.latestAt?.slice(0, 10) ?? "Unknown end"}
          </small>
          {!!preview.data.warnings.length && <p>{preview.data.warnings.length} non-blocking warnings</p>}
          <Button onClick={() => imported.mutate()} disabled={imported.isPending}>
            {imported.isPending ? "Importing…" : "Import normalized history"}
          </Button>
        </div>
      )}
      {imported.data && (
        <p className="success-note">
          <CheckCircle2 size={14} /> Imported {imported.data.imported}; skipped {imported.data.skipped}.
        </p>
      )}
      {error && (
        <div className="inline-error">
          <strong>Archive not imported</strong>
          <span>{error.message}</span>
        </div>
      )}
    </section>
  )
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
