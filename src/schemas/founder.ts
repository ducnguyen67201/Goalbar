import { z } from "zod"

export const founderInputSchema = z.object({
  name: z.string().trim().min(1).max(120),
  productName: z.string().trim().min(1).max(160),
  offer: z.string().trim().min(1).max(2000),
  expertise: z.string().trim().min(1).max(4000),
  goals: z.array(z.string().trim().min(1)),
  boundaries: z.array(z.string().trim().min(1)),
})

export const founderProfileSchema = founderInputSchema.extend({
  id: z.string().uuid(),
  onboardingCompleted: z.boolean(),
  createdAt: z.string(),
  updatedAt: z.string(),
})

export type FounderInput = z.infer<typeof founderInputSchema>
export type FounderProfile = z.infer<typeof founderProfileSchema>
