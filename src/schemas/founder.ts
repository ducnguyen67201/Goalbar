import { z } from "zod"

const websiteUrlSchema = z
  .string()
  .trim()
  .max(2048)
  .refine(
    (value) => value.length === 0 || /^https?:\/\/[^.\s]+\.[^\s]+$/i.test(value),
    "Enter a complete URL, including https://",
  )

const founderFieldsSchema = z.object({
  name: z.string().trim().min(1).max(120),
  productName: z.string().trim().min(1).max(160),
  websiteUrl: websiteUrlSchema,
  offer: z.string().trim().max(2000),
  idealCustomer: z.string().trim().min(1, "Tell us who you want to reach.").max(2000),
  expertise: z.string().trim().max(4000),
  goals: z.array(z.string().trim().min(1)),
  boundaries: z.array(z.string().trim().min(1)),
})

export const founderInputSchema = founderFieldsSchema.refine(
  (value) => value.websiteUrl.length > 0 || value.offer.length > 0,
  {
    path: ["offer"],
    message: "Add a landing page or a short description.",
  },
)

export const founderProfileSchema = founderFieldsSchema
  .omit({ websiteUrl: true, idealCustomer: true })
  .extend({
    websiteUrl: z.url().nullable(),
    idealCustomer: z.string().max(2000),
    id: z.string().uuid(),
    onboardingCompleted: z.boolean(),
    createdAt: z.string(),
    updatedAt: z.string(),
  })

export const updateFounderInputSchema = z.object({
  founderId: z.string().uuid(),
  profile: founderInputSchema,
})

export type FounderInput = z.infer<typeof founderInputSchema>
export type FounderProfile = z.infer<typeof founderProfileSchema>
