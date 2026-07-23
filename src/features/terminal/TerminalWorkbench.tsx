import { useMutation } from "@tanstack/react-query"
import { ChevronDown, ChevronUp, Code2, Plus, SquareTerminal, X } from "lucide-react"
import { useCallback, useEffect, useState } from "react"
import { z } from "zod"

import { Button } from "@/components/ui/button"
import { TerminalPane } from "@/features/terminal/TerminalPane"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  closeTerminalSessionInputSchema,
  createTerminalSessionInputSchema,
  terminalSessionSchema,
  type TerminalKind,
  type TerminalSession,
} from "@/schemas/terminal"

const sessionListSchema = z.array(terminalSessionSchema)

export function TerminalWorkbench() {
  const [sessions, setSessions] = useState<TerminalSession[]>([])
  const [collapsed, setCollapsed] = useState(false)

  useEffect(() => {
    if (!isTauriRuntime()) return
    void invokeOutput("list_terminal_sessions", {}, sessionListSchema)
      .then(setSessions)
      .catch(() => undefined)
  }, [])

  const create = useMutation({
    mutationFn: async (kind: TerminalKind) => {
      const input = { kind, rows: 20, cols: 80 }
      if (!isTauriRuntime()) {
        return terminalSessionSchema.parse({
          id: crypto.randomUUID(),
          kind,
          title: kind === "bash" ? "Shell" : kind === "codex" ? "Codex" : "Claude",
          status: "running",
          workingDirectory: "Local workspace",
          createdAt: new Date().toISOString(),
        })
      }
      return invokeValidated(
        "create_terminal_session",
        { input },
        createTerminalSessionInputSchema,
        terminalSessionSchema,
      )
    },
    onSuccess: (session) => {
      setSessions((current) => [...current.filter((item) => item.id !== session.id), session])
      setCollapsed(false)
    },
  })

  const close = useMutation({
    mutationFn: async (sessionId: string) => {
      const input = { sessionId }
      if (isTauriRuntime()) {
        await invokeValidated(
          "close_terminal_session",
          { input },
          closeTerminalSessionInputSchema,
          z.boolean(),
        )
      }
      return sessionId
    },
    onSuccess: (sessionId) => setSessions((current) => current.filter((session) => session.id !== sessionId)),
  })

  const handleExit = useCallback((sessionId: string, status: TerminalSession["status"]) => {
    setSessions((current) =>
      current.map((session) => (session.id === sessionId ? { ...session, status } : session)),
    )
  }, [])

  return (
    <section
      className={`terminal-workbench ${collapsed ? "collapsed" : ""}`}
      aria-label="Local agent terminals"
    >
      <header className="terminal-workbench-header">
        <div>
          <SquareTerminal size={15} />
          <strong>Local workbench</strong>
          <span>{sessions.length}/4 panes</span>
        </div>
        <div className="terminal-workbench-actions">
          {(["bash", "codex", "claude"] as const).map((kind) => (
            <Button
              key={kind}
              variant="secondary"
              size="small"
              disabled={sessions.length >= 4 || create.isPending}
              onClick={() => create.mutate(kind)}
            >
              {kind === "bash" ? <Plus size={12} /> : <Code2 size={12} />}
              {kind === "bash" ? "Shell" : kind}
            </Button>
          ))}
          {sessions.length > 0 && (
            <button
              className="terminal-icon-button"
              aria-label="Close every terminal"
              onClick={() => sessions.forEach((session) => close.mutate(session.id))}
            >
              <X size={14} />
            </button>
          )}
          <button
            className="terminal-icon-button"
            aria-label={collapsed ? "Expand local workbench" : "Collapse local workbench"}
            onClick={() => setCollapsed((value) => !value)}
          >
            {collapsed ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
        </div>
      </header>
      {!collapsed && (
        <div className="terminal-grid">
          {sessions.length === 0 ? (
            <div className="terminal-empty">
              <SquareTerminal size={18} />
              <span>Open a shell or an authenticated agent CLI. Nothing runs until you choose it.</span>
            </div>
          ) : (
            sessions.map((session) => (
              <TerminalPane
                key={session.id}
                session={session}
                onClose={(sessionId) => close.mutate(sessionId)}
                onExit={handleExit}
              />
            ))
          )}
        </div>
      )}
      {create.error && <div className="terminal-error">{create.error.message}</div>}
    </section>
  )
}
