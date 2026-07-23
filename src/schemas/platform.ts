import { z } from "zod"

import { capabilityStateSchema, platformSchema } from "./common"

export const platformCapabilitiesSchema = z.object({
  authenticate: capabilityStateSchema,
  publish: capabilityStateSchema,
  readOwnContent: capabilityStateSchema,
  metrics: capabilityStateSchema,
  reply: capabilityStateSchema,
  directMessages: capabilityStateSchema,
  detail: z.string().nullable().optional(),
})

export const connectedAccountSchema = z.object({
  id: z.string().uuid(),
  platform: platformSchema,
  clientId: z.string(),
  remoteAccountId: z.string(),
  displayName: z.string(),
  secretRef: z.string(),
  scopes: z.array(z.string()),
  capabilities: platformCapabilitiesSchema,
  tokenExpiresAt: z.string().nullable().optional(),
  status: z.string(),
})

export const connectedAccountsSchema = z.array(connectedAccountSchema)

export const beginOAuthInputSchema = z.object({
  platform: platformSchema,
  clientId: z.string().trim().min(1),
  remoteAccountId: z.string().trim().min(1),
  displayName: z.string().trim().min(1),
  scopes: z.array(z.string()),
})

export const beginOAuthResponseSchema = z.object({
  sessionId: z.string().uuid(),
  authorizationUrl: z.url(),
  redirectUri: z.url(),
  expiresAt: z.string(),
})

export const oauthStatusSchema = z.object({
  sessionId: z.string().uuid(),
  status: z.enum(["waiting_for_browser", "code_received", "complete", "failed", "expired"]),
  error: z.string().nullable().optional(),
})

export type ConnectedAccount = z.infer<typeof connectedAccountSchema>
export type BeginOAuthInput = z.infer<typeof beginOAuthInputSchema>
