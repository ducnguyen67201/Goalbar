import { useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, isTauriRuntime } from "@/lib/tauri"
import { bootstrapSchema, type BootstrapState } from "@/schemas/bootstrap"

const previewState: BootstrapState = {
  schemaVersion: 1,
  founder: null,
  agents: [
    { provider: "codex", readiness: "ready", version: "preview" },
    { provider: "claude", readiness: "ready", version: "preview" },
  ],
  accounts: [],
  score: {
    formulaVersion: 1,
    score: 0,
    confidence: 0,
    components: {},
    missing: [
      "attentionQuality",
      "conversationQuality",
      "relationshipGrowth",
      "consistency",
      "learningVelocity",
    ],
  },
  nextActions: [
    {
      kind: "onboarding",
      title: "Teach the lab who you are",
      reason: "Your ICP and voice need a founder baseline.",
      route: "/onboarding",
      priority: 100,
    },
  ],
}

export function useBootstrap() {
  return useQuery({
    queryKey: queryKeys.bootstrap,
    queryFn: () =>
      isTauriRuntime()
        ? invokeOutput("get_bootstrap_state", {}, bootstrapSchema)
        : Promise.resolve(previewState),
  })
}
