import { z } from "zod"

import { capabilityStateSchema, platformSchema } from "./common"

export const conversationSchema = z.object({
  id: z.string().uuid(),
  platform: platformSchema,
  remoteId: z.string(),
  kind: z.enum(["comment_thread", "direct_message"]),
  displayName: z.string(),
  preview: z.string(),
  unreadCount: z.number().int().nonnegative(),
  replyCapability: capabilityStateSchema,
  remoteUrl: z.string().nullable().optional(),
})

export const conversationsSchema = z.array(conversationSchema)
export const replyOptionsSchema = z.object({ options: z.array(z.string().min(1)).min(1) })
export const remoteMessageSchema = z.object({
  platform: platformSchema,
  remoteId: z.string(),
  conversationId: z.string().nullable().optional(),
  body: z.string(),
})
export type Conversation = z.infer<typeof conversationSchema>
