import { z } from "zod"

export const agentProviderSchema = z.enum(["codex", "claude"])
export const agentReadinessSchema = z.enum(["missing", "installed", "auth_required", "ready", "incompatible"])

export const agentStatusSchema = z.object({
  provider: agentProviderSchema,
  readiness: agentReadinessSchema,
  path: z.string().nullable().optional(),
  version: z.string().nullable().optional(),
  detail: z.string().nullable().optional(),
})

export const agentStatusesSchema = z.array(agentStatusSchema)
export type AgentProvider = z.infer<typeof agentProviderSchema>
export type AgentStatus = z.infer<typeof agentStatusSchema>
