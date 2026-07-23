import { z } from "zod"

import { platformSchema } from "./common"

export const historySelectionSchema = z
  .object({
    selectionId: z.string().uuid(),
    displayName: z.string(),
    sizeBytes: z.number().int().nonnegative(),
    container: z.string(),
    expiresAt: z.string().datetime({ offset: true }),
  })
  .strict()

export const historySelectionInputSchema = z.object({ selectionId: z.string().uuid() }).strict()

export const historyCategoryCountSchema = z
  .object({
    category: z.string(),
    count: z.number().int().nonnegative(),
  })
  .strict()

export const historyWarningSchema = z
  .object({
    code: z.string(),
    message: z.string(),
    member: z.string().nullable().optional(),
    row: z.number().int().nonnegative().nullable().optional(),
  })
  .strict()

export const historyPreviewSchema = z
  .object({
    schemaVersion: z.literal(1),
    selectionId: z.string().uuid(),
    platform: platformSchema,
    parserVersion: z.string(),
    displayName: z.string(),
    accountHandle: z.string().nullable().optional(),
    categories: z.array(historyCategoryCountSchema),
    estimatedRecords: z.number().int().nonnegative(),
    earliestAt: z.string().datetime({ offset: true }).nullable().optional(),
    latestAt: z.string().datetime({ offset: true }).nullable().optional(),
    warnings: z.array(historyWarningSchema),
    unsupportedMembers: z.array(z.string()),
    sourceFingerprint: z.string(),
  })
  .strict()

export const historyImportResultSchema = z
  .object({
    sourceId: z.string().uuid(),
    runId: z.string().uuid(),
    platform: platformSchema,
    imported: z.number().int().nonnegative(),
    skipped: z.number().int().nonnegative(),
    warningCount: z.number().int().nonnegative(),
    duplicateSource: z.boolean(),
  })
  .strict()

export const historyOverviewPlatformSchema = z
  .object({
    platform: platformSchema,
    sourceCount: z.number().int().nonnegative(),
    itemCount: z.number().int().nonnegative(),
    ownItemCount: z.number().int().nonnegative(),
    referenceItemCount: z.number().int().nonnegative(),
    latestAt: z.string().datetime({ offset: true }).nullable().optional(),
  })
  .strict()

export const historyOverviewSchema = z
  .object({
    schemaVersion: z.literal(1),
    sourceCount: z.number().int().nonnegative(),
    itemCount: z.number().int().nonnegative(),
    platforms: z.array(historyOverviewPlatformSchema),
  })
  .strict()

export type HistorySelection = z.infer<typeof historySelectionSchema>
export type HistoryPreview = z.infer<typeof historyPreviewSchema>
export type HistoryImportResult = z.infer<typeof historyImportResultSchema>
export type HistoryOverview = z.infer<typeof historyOverviewSchema>
