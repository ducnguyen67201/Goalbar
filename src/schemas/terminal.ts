import { z } from "zod"

export const terminalKindSchema = z.enum(["bash", "codex", "claude"])
export const terminalStatusSchema = z.enum(["running", "exited", "failed"])

export const terminalSessionSchema = z
  .object({
    id: z.string().uuid(),
    kind: terminalKindSchema,
    title: z.string().min(1),
    status: terminalStatusSchema,
    workingDirectory: z.string(),
    createdAt: z.string().datetime({ offset: true }),
  })
  .strict()

export const terminalOutputEventSchema = z
  .object({
    sessionId: z.string().uuid(),
    data: z.string(),
  })
  .strict()

export const terminalExitEventSchema = z
  .object({
    sessionId: z.string().uuid(),
    status: terminalStatusSchema,
    exitCode: z.number().int().nonnegative().nullable().optional(),
  })
  .strict()

export const createTerminalSessionInputSchema = z
  .object({
    kind: terminalKindSchema,
    rows: z.number().int().min(2).max(500),
    cols: z.number().int().min(2).max(500),
  })
  .strict()

export const writeTerminalSessionInputSchema = z
  .object({
    sessionId: z.string().uuid(),
    data: z.string().max(64 * 1024),
  })
  .strict()

export const resizeTerminalSessionInputSchema = z
  .object({
    sessionId: z.string().uuid(),
    rows: z.number().int().min(2).max(500),
    cols: z.number().int().min(2).max(500),
  })
  .strict()

export const closeTerminalSessionInputSchema = z
  .object({
    sessionId: z.string().uuid(),
  })
  .strict()

export type TerminalKind = z.infer<typeof terminalKindSchema>
export type TerminalSession = z.infer<typeof terminalSessionSchema>
