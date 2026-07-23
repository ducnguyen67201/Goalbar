import { z } from "zod"

export const growthInputsSchema = z.object({
  attentionQuality: z.number().nullable().optional(),
  conversationQuality: z.number().nullable().optional(),
  relationshipGrowth: z.number().nullable().optional(),
  consistency: z.number().nullable().optional(),
  learningVelocity: z.number().nullable().optional(),
})

export const growthScoreSchema = z.object({
  formulaVersion: z.number().int(),
  score: z.number(),
  confidence: z.number(),
  components: growthInputsSchema,
  missing: z.array(z.string()),
})

export const nextActionSchema = z.object({
  kind: z.string(),
  title: z.string(),
  reason: z.string(),
  route: z.string(),
  priority: z.number().int(),
})

export const weeklyLearningSchema = z.object({
  observation: z.string(),
  learning: z.string(),
  counterEvidence: z.array(z.string()),
  confidence: z.number(),
  nextExperiment: z.string(),
})

export type GrowthScore = z.infer<typeof growthScoreSchema>
