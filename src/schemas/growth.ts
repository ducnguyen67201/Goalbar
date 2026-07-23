import { z } from "zod"

import { platformSchema } from "./common"
import { storedIcpHypothesisSchema } from "./memory"

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

export const growthActionKindSchema = z.enum(["research", "follow", "comment", "post"])
export const growthActionStatusSchema = z.enum([
  "proposed",
  "approved",
  "completed",
  "failed",
  "cancelled",
  "measured",
])
export const executionOutcomeSchema = z.enum(["succeeded", "failed"])
export const metricAvailabilitySchema = z.enum(["available", "missing", "restricted", "delayed"])

export const growthActionExecutionSchema = z.object({
  id: z.string().uuid(),
  actionId: z.string().uuid(),
  approvalId: z.string().uuid(),
  outcome: executionOutcomeSchema,
  resultUrl: z.string().url().nullable().optional(),
  detail: z.string(),
  attemptedAt: z.string(),
})

export const growthActionMetricSchema = z.object({
  id: z.string().uuid(),
  actionId: z.string().uuid(),
  metricName: z.string(),
  value: z.number().nullable().optional(),
  availability: metricAvailabilitySchema,
  sourceDefinition: z.string(),
  notes: z.string(),
  observedAt: z.string(),
  collectedAt: z.string(),
})

export const growthActionSchema = z.object({
  id: z.string().uuid(),
  founderId: z.string().uuid(),
  icpHypothesisId: z.string().uuid().nullable().optional(),
  experimentId: z.string().uuid().nullable().optional(),
  kind: growthActionKindSchema,
  platform: platformSchema.nullable().optional(),
  title: z.string(),
  rationale: z.string(),
  targetUrl: z.string().url().nullable().optional(),
  exactPayload: z.string(),
  payloadHash: z.string(),
  revision: z.number().int().positive(),
  hypothesis: z.string(),
  successMetric: z.string(),
  evaluationWindowDays: z.number().int().min(1).max(365),
  status: growthActionStatusSchema,
  scheduledFor: z.string().nullable().optional(),
  completedAt: z.string().nullable().optional(),
  createdAt: z.string(),
  updatedAt: z.string(),
  approvalId: z.string().uuid().nullable().optional(),
  executions: z.array(growthActionExecutionSchema),
  metrics: z.array(growthActionMetricSchema),
})

export const trackedGrowthLearningSchema = z.object({
  id: z.string().uuid(),
  growthActionId: z.string().uuid().nullable().optional(),
  summary: z.string(),
  evidence: z.unknown(),
  confidence: z.number().min(0).max(1),
  status: z.string(),
  createdAt: z.string(),
})

export const growthLoopOverviewSchema = z.object({
  schemaVersion: z.literal(1),
  activeIcp: storedIcpHypothesisSchema.nullable(),
  actions: z.array(growthActionSchema),
  learnings: z.array(trackedGrowthLearningSchema),
  totals: z.object({
    proposed: z.number().int().nonnegative(),
    approved: z.number().int().nonnegative(),
    completed: z.number().int().nonnegative(),
    measured: z.number().int().nonnegative(),
  }),
})

export const proposeGrowthActionInputSchema = z
  .object({
    icpHypothesisId: z.string().uuid().nullable().optional(),
    experimentId: z.string().uuid().nullable().optional(),
    kind: growthActionKindSchema,
    platform: platformSchema.nullable().optional(),
    title: z.string().trim().min(1).max(200),
    rationale: z.string().trim().min(1).max(2000),
    targetUrl: z.string().url().nullable().optional(),
    exactPayload: z.string().trim().min(1).max(40000),
    hypothesis: z.string().trim().min(1).max(2000),
    successMetric: z.string().trim().min(1).max(1000),
    evaluationWindowDays: z.number().int().min(1).max(365),
    scheduledFor: z.string().nullable().optional(),
  })
  .strict()

export const approveGrowthActionInputSchema = z
  .object({
    actionId: z.string().uuid(),
    exactPayload: z.string().min(1).max(40000),
  })
  .strict()

export const reviseGrowthActionInputSchema = approveGrowthActionInputSchema

export const recordGrowthActionExecutionInputSchema = z
  .object({
    actionId: z.string().uuid(),
    approvalId: z.string().uuid(),
    exactPayload: z.string().min(1).max(40000),
    outcome: executionOutcomeSchema,
    resultUrl: z.string().url().nullable().optional(),
    detail: z.string().trim().min(1).max(2000),
  })
  .strict()

export const recordGrowthActionMetricInputSchema = z
  .object({
    actionId: z.string().uuid(),
    metricName: z
      .string()
      .trim()
      .min(1)
      .max(100)
      .regex(/^[a-z0-9_]+$/),
    value: z.number().nonnegative().nullable().optional(),
    availability: metricAvailabilitySchema,
    sourceDefinition: z.string().trim().min(1).max(1000),
    notes: z.string().max(2000),
    observedAt: z.string(),
  })
  .strict()
  .superRefine((value, context) => {
    if (value.availability === "available" && value.value == null) {
      context.addIssue({ code: "custom", path: ["value"], message: "Available metrics need a value" })
    }
    if (value.availability !== "available" && value.value != null) {
      context.addIssue({
        code: "custom",
        path: ["value"],
        message: "Unavailable metrics cannot include a value",
      })
    }
  })

export const recordGrowthLearningInputSchema = z
  .object({
    actionId: z.string().uuid(),
    observation: z.string().trim().min(1).max(4000),
    learning: z.string().trim().min(1).max(4000),
    counterEvidence: z.array(z.string().max(1000)).max(20),
    confidence: z.number().min(0).max(1),
    nextExperiment: z.string().trim().min(1).max(2000),
  })
  .strict()

export type GrowthScore = z.infer<typeof growthScoreSchema>
export type GrowthAction = z.infer<typeof growthActionSchema>
export type GrowthLoopOverview = z.infer<typeof growthLoopOverviewSchema>
export type ProposeGrowthActionInput = z.infer<typeof proposeGrowthActionInputSchema>
