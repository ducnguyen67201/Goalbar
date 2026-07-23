import { listen } from "@tauri-apps/api/event"
import { X } from "lucide-react"
import { useEffect, useRef } from "react"
import { z } from "zod"

import { invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  resizeTerminalSessionInputSchema,
  terminalExitEventSchema,
  terminalOutputEventSchema,
  type TerminalSession,
  writeTerminalSessionInputSchema,
} from "@/schemas/terminal"

type TerminalPaneProps = {
  session: TerminalSession
  onClose: (sessionId: string) => void
  onExit: (sessionId: string, status: TerminalSession["status"]) => void
}

export function TerminalPane({ session, onClose, onExit }: TerminalPaneProps) {
  const surfaceRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!isTauriRuntime() || !surfaceRef.current) return
    let disposed = false
    let cleanup = () => undefined
    const surface = surfaceRef.current
    void Promise.all([import("@xterm/xterm"), import("@xterm/addon-fit")]).then(
      ([{ Terminal }, { FitAddon }]) => {
        if (disposed) return
        const terminal = new Terminal({
          cursorBlink: true,
          convertEol: false,
          fontFamily: '"SFMono-Regular", "Cascadia Code", Menlo, monospace',
          fontSize: 11,
          lineHeight: 1.2,
          scrollback: 5_000,
          theme: {
            background: "#11120f",
            foreground: "#e7e8df",
            cursor: "#b7e34b",
            selectionBackground: "#53642f",
            black: "#11120f",
            brightBlack: "#6c6f65",
            green: "#9fc93d",
            brightGreen: "#b7e34b",
          },
        })
        const fit = new FitAddon()
        terminal.loadAddon(fit)
        terminal.open(surface)

        const resize = () => {
          fit.fit()
          const input = {
            sessionId: session.id,
            rows: Math.max(2, terminal.rows),
            cols: Math.max(2, terminal.cols),
          }
          void invokeValidated(
            "resize_terminal_session",
            { input },
            resizeTerminalSessionInputSchema,
            z.boolean(),
          ).catch(() => undefined)
        }
        const observer = new ResizeObserver(resize)
        observer.observe(surface)
        resize()

        const inputSubscription = terminal.onData((data) => {
          const input = { sessionId: session.id, data }
          void invokeValidated(
            "write_terminal_session",
            { input },
            writeTerminalSessionInputSchema,
            z.boolean(),
          ).catch((error: Error) => terminal.writeln(`\r\n[Goalbar] ${error.message}`))
        })
        const outputListener = listen<unknown>("terminal://output", (event) => {
          const output = terminalOutputEventSchema.parse(event.payload)
          if (output.sessionId === session.id) terminal.write(output.data)
        })
        const exitListener = listen<unknown>("terminal://exit", (event) => {
          const exit = terminalExitEventSchema.parse(event.payload)
          if (exit.sessionId !== session.id) return
          terminal.writeln(
            `\r\n\x1b[38;2;183;227;75m[Goalbar] Process ${exit.status}${exit.exitCode === undefined || exit.exitCode === null ? "" : ` (${exit.exitCode})`}.\x1b[0m`,
          )
          onExit(session.id, exit.status)
        })
        cleanup = () => {
          observer.disconnect()
          inputSubscription.dispose()
          void outputListener.then((dispose) => dispose())
          void exitListener.then((dispose) => dispose())
          terminal.dispose()
        }
      },
    )
    return () => {
      disposed = true
      cleanup()
    }
  }, [onExit, session.id])

  return (
    <article className="terminal-pane">
      <header>
        <span className={`terminal-status ${session.status}`} />
        <strong>{session.title}</strong>
        <small title={session.workingDirectory}>{session.workingDirectory}</small>
        <button aria-label={`Close ${session.title}`} onClick={() => onClose(session.id)}>
          <X size={13} />
        </button>
      </header>
      {isTauriRuntime() ? (
        <div className="terminal-surface" ref={surfaceRef} />
      ) : (
        <div className="terminal-preview">
          <span>$ local {session.kind} session</span>
          <span className="cursor">▋</span>
        </div>
      )}
    </article>
  )
}
