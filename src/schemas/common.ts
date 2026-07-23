import { z } from "zod"

export const commandErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  recovery: z.string().nullable().optional(),
})

export type CommandError = z.infer<typeof commandErrorSchema>

export const platformSchema = z.enum(["x", "reddit", "linkedin"])
export const capabilityStateSchema = z.enum(["supported", "unsupported", "approval_pending", "unknown"])

export const emptyInputSchema = z.object({}).strict()

export const dataArtifactSchema = z.object({
  path: z.string(),
  kind: z.string(),
  createdAt: z.string(),
  includesSecrets: z.literal(false),
})
