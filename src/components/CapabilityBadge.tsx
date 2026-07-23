import { Badge } from "@/components/ui/badge"
import { titleCase } from "@/lib/utils"
type Capability = "supported" | "unsupported" | "approval_pending" | "unknown"

export function CapabilityBadge({ state }: { state: Capability }) {
  const tone = state === "supported" ? "good" : state === "unsupported" ? "neutral" : "warn"
  return <Badge tone={tone}>{titleCase(state)}</Badge>
}
