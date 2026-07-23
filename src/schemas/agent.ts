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

export const founderChatResearchRequestSchema = z
  .object({
    objective: z.string().trim().min(1).max(1_000),
    reason: z.string().trim().min(1).max(1_000),
    ownership: z.enum(["own", "reference"]),
    maximumItems: z.number().int().min(1).max(500),
    maximumSteps: z.number().int().min(1).max(100),
  })
  .strict()

export const founderChatTurnSchema = z
  .object({
    reply: z.string().trim().min(1).max(8_000),
    researchRequest: founderChatResearchRequestSchema.nullable(),
  })
  .strict()

export const runAgentTaskInputSchema = z
  .object({
    provider: agentProviderSchema,
    taskKind: z.string().trim().min(1).max(100),
    prompt: z.string().trim().min(1).max(8_000),
    context: z.record(z.string(), z.unknown()),
    outputSchema: z.record(z.string(), z.unknown()),
  })
  .strict()

export const founderChatAgentResultSchema = z
  .object({
    provider: agentProviderSchema,
    providerVersion: z.string(),
    output: founderChatTurnSchema,
    usage: z.unknown().nullable().optional(),
  })
  .strict()

export const founderChatOutputJsonSchema = {
  type: "object",
  additionalProperties: false,
  required: ["reply", "researchRequest"],
  properties: {
    reply: { type: "string", minLength: 1, maxLength: 8_000 },
    researchRequest: {
      anyOf: [
        {
          type: "object",
          additionalProperties: false,
          required: ["objective", "reason", "ownership", "maximumItems", "maximumSteps"],
          properties: {
            objective: { type: "string", minLength: 1, maxLength: 1_000 },
            reason: { type: "string", minLength: 1, maxLength: 1_000 },
            ownership: { type: "string", enum: ["own", "reference"] },
            maximumItems: { type: "integer", minimum: 1, maximum: 500 },
            maximumSteps: { type: "integer", minimum: 1, maximum: 100 },
          },
        },
        { type: "null" },
      ],
    },
  },
} as const

export type AgentProvider = z.infer<typeof agentProviderSchema>
export type AgentStatus = z.infer<typeof agentStatusSchema>
export type FounderChatResearchRequest = z.infer<typeof founderChatResearchRequestSchema>
export type FounderChatTurn = z.infer<typeof founderChatTurnSchema>
