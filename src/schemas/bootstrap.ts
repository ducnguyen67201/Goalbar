import { z } from "zod"

import { agentStatusSchema } from "./agent"
import { founderProfileSchema } from "./founder"
import { growthScoreSchema, nextActionSchema } from "./growth"
import { connectedAccountSchema } from "./platform"

export const bootstrapSchema = z.object({
  schemaVersion: z.literal(1),
  founder: founderProfileSchema.nullable(),
  agents: z.array(agentStatusSchema),
  accounts: z.array(connectedAccountSchema),
  score: growthScoreSchema,
  nextActions: z.array(nextActionSchema),
})

export type BootstrapState = z.infer<typeof bootstrapSchema>
