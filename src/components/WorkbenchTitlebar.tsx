import { LockKeyhole } from "lucide-react"
import { useLocation } from "react-router-dom"

const routeTitles: Record<string, string> = {
  "/": "Today",
  "/browser": "Browser",
  "/create": "Create",
  "/inbox": "Inbox",
  "/growth": "Growth",
  "/memory": "Memory",
  "/settings": "Settings",
  "/onboarding": "Founder setup",
}

export function WorkbenchTitlebar() {
  const location = useLocation()
  const title = routeTitles[location.pathname] ?? "Goalbar"

  return (
    <header className="workbench-titlebar" data-tauri-drag-region>
      <span className="titlebar-product" data-tauri-drag-region>
        Goalbar
      </span>
      <span className="titlebar-divider" aria-hidden="true" />
      <strong data-tauri-drag-region>{title}</strong>
      <span className="titlebar-status" data-tauri-drag-region>
        <LockKeyhole size={12} aria-hidden="true" /> Local session
      </span>
    </header>
  )
}
