import { commandErrorSchema, type CommandError } from "@/schemas/common"

export class AppCommandError extends Error {
  readonly code: string
  readonly recovery?: string | null

  constructor(error: CommandError) {
    super(error.message)
    this.name = "AppCommandError"
    this.code = error.code
    this.recovery = error.recovery
  }
}

export function normalizeCommandError(error: unknown): AppCommandError {
  if (error instanceof AppCommandError) return error
  const parsed = commandErrorSchema.safeParse(error)
  if (parsed.success) return new AppCommandError(parsed.data)
  if (error instanceof Error) return new AppCommandError({ code: "unknown", message: error.message })
  return new AppCommandError({ code: "unknown", message: String(error) })
}
