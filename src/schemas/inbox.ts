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
  profileUrl: z.string().nullable().optional(),
  source: z.enum(["platform_api", "email_notification", "browser_scan"]),
  contentState: z.enum(["complete", "notification_excerpt", "link_only"]),
  updatedAt: z.string(),
})

export const conversationsSchema = z.array(conversationSchema)
export const browserInboxScanInputSchema = z.object({ platform: platformSchema })
export const browserInboxScanResultSchema = z.object({
  platform: platformSchema,
  status: z.enum([
    "completed",
    "partial",
    "needs_browser",
    "login_required",
    "verification_required",
    "unsupported_page",
  ]),
  scanned: z.number().int().nonnegative(),
  imported: z.number().int().nonnegative(),
  updated: z.number().int().nonnegative(),
  lastScannedAt: z.string(),
  message: z.string(),
  targetUrl: z.url(),
})
export const emailNotificationSyncResultSchema = z.object({
  source: z.literal("apple_mail"),
  scanned: z.number().int().nonnegative(),
  imported: z.number().int().nonnegative(),
  ignored: z.number().int().nonnegative(),
  duplicates: z.number().int().nonnegative(),
  platformCounts: z.object({
    x: z.number().int().nonnegative(),
    reddit: z.number().int().nonnegative(),
    linkedin: z.number().int().nonnegative(),
  }),
  lastCheckedAt: z.string(),
})
export const replyOptionsSchema = z.object({ options: z.array(z.string().min(1)).min(1) })
export const remoteMessageSchema = z.object({
  platform: platformSchema,
  remoteId: z.string(),
  conversationId: z.string().nullable().optional(),
  body: z.string(),
})
export type Conversation = z.infer<typeof conversationSchema>
export type BrowserInboxScanResult = z.infer<typeof browserInboxScanResultSchema>
export type EmailNotificationSyncResult = z.infer<typeof emailNotificationSyncResultSchema>
