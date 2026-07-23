import { invoke } from "@tauri-apps/api/core"
import type { ZodType } from "zod"

import { normalizeCommandError } from "./errors"

export const isTauriRuntime = () => "__TAURI_INTERNALS__" in window

export async function invokeValidated<Input, Output>(
  command: string,
  args: Record<string, unknown>,
  inputSchema: ZodType<Input>,
  outputSchema: ZodType<Output>,
): Promise<Output> {
  const inputKey = Object.keys(args)[0]
  const parsedInput = inputSchema.parse(inputKey ? args[inputKey] : {})
  const validatedArgs = inputKey ? { ...args, [inputKey]: parsedInput } : args
  try {
    const result = await invoke<unknown>(command, validatedArgs)
    return outputSchema.parse(result)
  } catch (error) {
    throw normalizeCommandError(error)
  }
}

export async function invokeOutput<Output>(
  command: string,
  args: Record<string, unknown>,
  outputSchema: ZodType<Output>,
) {
  try {
    return outputSchema.parse(await invoke<unknown>(command, args))
  } catch (error) {
    throw normalizeCommandError(error)
  }
}
