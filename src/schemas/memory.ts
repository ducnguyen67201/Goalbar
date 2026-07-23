import { z } from "zod"

export const voiceProfileInputSchema = z.object({
  traits: z.array(z.string().min(1)),
  doRules: z.array(z.string().min(1)),
  dontRules: z.array(z.string().min(1)),
  example: z.string().min(10),
})

export const saveVoiceInputSchema = z.object({
  founderId: z.string().uuid(),
  voice: voiceProfileInputSchema,
})

export const icpHypothesisSchema = z.object({
  role: z.string(),
  situation: z.string(),
  urgentProblem: z.string(),
  currentWorkaround: z.string(),
  desiredOutcome: z.string(),
  objections: z.array(z.string()),
  language: z.array(z.string()),
  confidence: z.number().min(0).max(1),
})

export const icpHypothesesSchema = z.object({ hypotheses: z.array(icpHypothesisSchema).min(1) })

export const storedIcpHypothesisSchema = icpHypothesisSchema.extend({
  id: z.string().uuid(),
  founderId: z.string().uuid(),
  version: z.number().int().positive(),
  parentId: z.string().uuid().nullable().optional(),
  status: z.enum(["proposed", "active", "rejected", "archived"]),
  createdAt: z.string(),
  updatedAt: z.string(),
})

export const storedIcpHypothesesSchema = z.array(storedIcpHypothesisSchema)
export const acceptIcpInputSchema = z.object({ hypothesisId: z.string().uuid() })
