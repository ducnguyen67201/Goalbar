import {
  BarChart3,
  BrainCircuit,
  Globe2,
  Inbox,
  LayoutDashboard,
  PenLine,
  Settings2,
  Sparkles,
} from "lucide-react"
import { NavLink, Outlet, useLocation } from "react-router-dom"

import { WorkbenchTitlebar } from "@/components/WorkbenchTitlebar"
import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

const navigation = [
  { to: "/", label: "Today", icon: LayoutDashboard, end: true },
  { to: "/browser", label: "Browser", icon: Globe2 },
  { to: "/create", label: "Create", icon: PenLine },
  { to: "/inbox", label: "Inbox", icon: Inbox },
  { to: "/growth", label: "Growth", icon: BarChart3 },
  { to: "/memory", label: "Memory", icon: BrainCircuit },
  { to: "/settings", label: "Settings", icon: Settings2 },
]

export function AppShell() {
  const location = useLocation()
  const browserRoute = location.pathname === "/browser"

  return (
    <div className={cn("app-frame", browserRoute && "app-frame-browser")}>
      <WorkbenchTitlebar />
      <aside className="sidebar">
        <NavLink className="brand" to="/" aria-label="Tagline home">
          <span className="brand-mark">
            <Sparkles size={18} />
          </span>
          <span>
            <strong>Tagline</strong>
            <small>Founder OS</small>
          </span>
        </NavLink>
        <nav aria-label="Main navigation">
          {navigation.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) => cn("nav-item", isActive && "nav-item-active")}
            >
              <Icon size={17} aria-hidden="true" />
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-foot">
          <Badge tone="good">Local only</Badge>
          <p>Your memory, website sessions, and platform tokens stay on this machine.</p>
        </div>
      </aside>
      <main className="main-content">
        <Outlet />
      </main>
    </div>
  )
}
