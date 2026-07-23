import { z } from "zod"

import { agentProviderSchema } from "./agent"
import { platformSchema } from "./common"

export const contentIdeaSchema = z.object({
  title: z.string().trim().min(1).max(200),
  insight: z.string().trim().min(10).max(10000),
  hypothesis: z.string().trim().min(1).max(2000),
  successMetric: z.string().trim().min(1).max(1000),
})

export const generateContentInputSchema = z.object({
  provider: agentProviderSchema,
  idea: contentIdeaSchema,
})

export const storedContentVariantSchema = z.object({
  id: z.string().uuid(),
  platform: platformSchema,
  revision: z.number().int().positive(),
  body: z.string(),
  status: z.string(),
})

export const generateContentResponseSchema = z.object({
  ideaId: z.string().uuid(),
  variants: z.array(storedContentVariantSchema).length(3),
})

export const approvalSchema = z.object({
  id: z.string().uuid(),
  subjectType: z.string(),
  subjectId: z.string().uuid(),
  payloadHash: z.string(),
  idempotencyKey: z.string().uuid(),
  approvedAt: z.string(),
  consumedAt: z.string().nullable().optional(),
  invalidatedAt: z.string().nullable().optional(),
})

export const approveVariantInputSchema = z.object({
  variantId: z.string().uuid(),
  body: z.string().min(1),
})

export const publishVariantInputSchema = z.object({
  accountId: z.string().uuid(),
  approvalId: z.string().uuid(),
  variantId: z.string().uuid(),
  body: z.string().min(1),
  title: z.string().nullable().optional(),
  destination: z.string().nullable().optional(),
})

export const remoteContentSchema = z.object({
  platform: platformSchema,
  remoteId: z.string(),
  body: z.string(),
  remoteUrl: z.string().nullable().optional(),
})

export type ContentIdea = z.infer<typeof contentIdeaSchema>
export type StoredContentVariant = z.infer<typeof storedContentVariantSchema>
