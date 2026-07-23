import type { ReactNode } from "react"

import { cn } from "@/lib/utils"

export function Badge({
  children,
  tone = "neutral",
  className,
}: {
  children: ReactNode
  tone?: "good" | "warn" | "neutral" | "danger"
  className?: string
}) {
  return <span className={cn("badge", `badge-${tone}`, className)}>{children}</span>
}
