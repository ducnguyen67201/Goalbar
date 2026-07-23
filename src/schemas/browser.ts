import { z } from "zod"

import { platformSchema } from "./common"

export const browserLoadStateSchema = z.enum(["idle", "loading", "loaded", "failed"])
export const browserPageKindSchema = z.enum([
  "feed",
  "profile",
  "post",
  "messages",
  "search",
  "login",
  "challenge",
  "unknown",
])
export const browserPolicyStateSchema = z.enum([
  "explicit_capture",
  "bounded_collection",
  "manual_only",
  "blocked",
])
export const browserRunStatusSchema = z.enum(["running", "paused", "completed", "cancelled", "failed"])
export const browserPauseReasonSchema = z.enum([
  "login_required",
  "verification_required",
  "rate_limited",
  "unsupported_page",
  "host_changed",
  "policy_restricted",
  "uncertain",
])
export const browserReplyPreparationStatusSchema = z.enum([
  "prepared",
  "composer_not_found",
  "login_required",
  "verification_required",
  "unsupported_page",
])
export const savedBrowserReplyStatusSchema = z.enum(["prepared", "confirmed_posted"])

export const browserBoundsSchema = z
  .object({
    x: z.number().finite().nonnegative(),
    y: z.number().finite().nonnegative(),
    width: z.number().finite().min(80).max(10_000),
    height: z.number().finite().min(80).max(10_000),
  })
  .strict()

export const browserTabSchema = z
  .object({
    id: z.string().uuid(),
    webviewLabel: z.string(),
    currentUrl: z.string().url(),
    title: z.string(),
    loadState: browserLoadStateSchema,
    platform: platformSchema.nullable().optional(),
    active: z.boolean(),
    createdAt: z.string().datetime({ offset: true }),
  })
  .strict()

export const savedBrowserReplySchema = z
  .object({
    id: z.string().uuid(),
    platform: platformSchema,
    targetUrl: z.string().url(),
    exactReply: z.string().min(1).max(8_000),
    status: savedBrowserReplyStatusSchema,
    preparedAt: z.string().datetime({ offset: true }),
    confirmedPostedAt: z.string().datetime({ offset: true }).nullable().optional(),
  })
  .strict()

export const browserReplyPreparationSchema = z
  .object({
    status: browserReplyPreparationStatusSchema,
    platform: platformSchema.nullable().optional(),
    characterCount: z.number().int().nonnegative(),
    savedReply: savedBrowserReplySchema.nullable().optional(),
  })
  .strict()

export const browserObservationBlockSchema = z
  .object({
    key: z.string(),
    role: z.string(),
    text: z.string(),
    links: z.array(z.string().url()),
    timestamp: z.string().datetime({ offset: true }).nullable().optional(),
  })
  .strict()

export const browserObservationSchema = z
  .object({
    schemaVersion: z.literal(1),
    tabId: z.string().uuid(),
    url: z.string().url(),
    title: z.string(),
    platform: platformSchema.nullable().optional(),
    pageKind: browserPageKindSchema,
    viewport: z
      .object({
        width: z.number().int().nonnegative(),
        height: z.number().int().nonnegative(),
        scrollY: z.number().nonnegative(),
      })
      .strict(),
    visibleBlocks: z.array(browserObservationBlockSchema),
    capturedItemKeys: z.array(z.string()),
    warning: z.string().nullable().optional(),
  })
  .strict()

export const browserCapturePreviewSchema = z
  .object({
    observation: browserObservationSchema,
    selectedText: z.string().nullable().optional(),
    normalizedItemCount: z.number().int().nonnegative(),
    policyState: browserPolicyStateSchema,
  })
  .strict()

export const browserNewWindowRequestSchema = z
  .object({
    url: z.string().url(),
  })
  .strict()

export const browserRunLimitsSchema = z
  .object({
    maximumItems: z.number().int().min(1).max(500),
    maximumSteps: z.number().int().min(1).max(100),
    earliestDate: z.string().datetime().nullable().optional(),
  })
  .strict()

export const browserRunProgressSchema = z
  .object({
    runId: z.string().uuid(),
    status: browserRunStatusSchema,
    step: z.number().int().nonnegative(),
    itemCount: z.number().int().nonnegative(),
    newItemCount: z.number().int().nonnegative(),
    pauseReason: browserPauseReasonSchema.nullable().optional(),
    summary: z.string().nullable().optional(),
  })
  .strict()

export const researchFindingCategorySchema = z.enum([
  "pain",
  "goal",
  "objection",
  "language",
  "trigger",
  "content_theme",
  "counter_evidence",
])
export const researchFindingStatusSchema = z.enum(["proposed", "accepted", "rejected"])
export const storedResearchFindingSchema = z
  .object({
    id: z.string().uuid(),
    runId: z.string().uuid(),
    platform: platformSchema,
    category: researchFindingCategorySchema,
    summary: z.string(),
    evidenceExcerpt: z.string(),
    sourceUrl: z.string().url(),
    confidence: z.number().min(0).max(1),
    status: researchFindingStatusSchema,
    createdAt: z.string().datetime({ offset: true }),
    updatedAt: z.string().datetime({ offset: true }),
  })
  .strict()
export const browserResearchTraceSchema = z
  .object({
    id: z.string().uuid(),
    runId: z.string().uuid(),
    step: z.number().int().nonnegative(),
    action: z.enum(["observe", "scroll", "open_link", "go_back", "finish", "pause", "error"]),
    message: z.string(),
    url: z.string().url(),
    createdAt: z.string().datetime({ offset: true }),
  })
  .strict()

export const createBrowserTabInputSchema = z
  .object({
    url: z.string().url(),
    bounds: browserBoundsSchema,
  })
  .strict()
export const browserTabInputSchema = z.object({ tabId: z.string().uuid() }).strict()
export const browserBoundsInputSchema = z.object({ bounds: browserBoundsSchema }).strict()
export const navigateBrowserInputSchema = z
  .object({ tabId: z.string().uuid(), url: z.string().url() })
  .strict()
export const prepareBrowserReplyInputSchema = z
  .object({
    tabId: z.string().uuid(),
    url: z.url().refine((value) => {
      const parsed = new URL(value)
      const host = parsed.hostname.toLowerCase().replace(/\.$/, "")
      return (
        parsed.protocol === "https:" &&
        ["x.com", "twitter.com", "linkedin.com", "reddit.com"].some(
          (root) => host === root || host.endsWith(`.${root}`),
        )
      )
    }, "Reply preparation requires HTTPS on X, LinkedIn, or Reddit"),
    reply: z
      .string()
      .min(1)
      .max(8_000)
      .refine((value) => value.trim().length > 0, "The exact reply cannot be empty"),
  })
  .strict()
export const browserCaptureInputSchema = z
  .object({
    tabId: z.string().uuid(),
    mode: z.enum(["visible", "selection"]),
    ownership: z.enum(["own", "reference"]),
  })
  .strict()
export const startBrowserCollectionInputSchema = z
  .object({
    tabId: z.string().uuid(),
    objective: z.string().trim().min(1).max(1_000),
    limits: browserRunLimitsSchema,
    ownership: z.enum(["own", "reference"]),
    provider: z.enum(["codex", "claude"]).nullable().optional(),
  })
  .strict()
export const cancelBrowserCollectionInputSchema = z.object({ runId: z.string().uuid() }).strict()
export const reviewBrowserResearchFindingInputSchema = z
  .object({
    findingId: z.string().uuid(),
    status: z.enum(["accepted", "rejected"]),
  })
  .strict()
export const clearBrowserDataInputSchema = z
  .object({ confirmation: z.literal("CLEAR BROWSER DATA") })
  .strict()
export const browserPanelWidthSchema = z.number().finite().min(280).max(480)
export const browserPanelWidthInputSchema = z.object({ width: browserPanelWidthSchema }).strict()

export type BrowserBounds = z.infer<typeof browserBoundsSchema>
export type BrowserTab = z.infer<typeof browserTabSchema>
export type BrowserReplyPreparation = z.infer<typeof browserReplyPreparationSchema>
export type SavedBrowserReply = z.infer<typeof savedBrowserReplySchema>
export type BrowserCapturePreview = z.infer<typeof browserCapturePreviewSchema>
export type BrowserRunProgress = z.infer<typeof browserRunProgressSchema>
export type BrowserCaptureInput = z.infer<typeof browserCaptureInputSchema>
export type StoredResearchFinding = z.infer<typeof storedResearchFindingSchema>
export type BrowserResearchTrace = z.infer<typeof browserResearchTraceSchema>
