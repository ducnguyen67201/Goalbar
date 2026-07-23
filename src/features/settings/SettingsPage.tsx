import { useMutation, useQueryClient } from "@tanstack/react-query"
import { Bot, CheckCircle2, ExternalLink, Globe2, KeyRound, Link2, LockKeyhole, Trash2 } from "lucide-react"
import { useMemo, useState } from "react"
import { Link } from "react-router-dom"
import { z } from "zod"

import { useBootstrap } from "@/app/bootstrap"
import { CapabilityBadge } from "@/components/CapabilityBadge"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { titleCase } from "@/lib/utils"
import { clearBrowserDataInputSchema } from "@/schemas/browser"
import { dataArtifactSchema } from "@/schemas/common"
import {
  beginOAuthInputSchema,
  beginOAuthResponseSchema,
  connectedAccountSchema,
  oauthStatusSchema,
  type BeginOAuthInput,
} from "@/schemas/platform"

const defaultScopes = {
  x: ["tweet.read", "tweet.write", "users.read", "dm.read", "dm.write", "offline.access"],
  reddit: ["identity", "read", "history", "submit", "edit", "privatemessages"],
  linkedin: ["openid", "profile", "w_member_social", "r_member_social"],
} as const

export function SettingsPage() {
  const bootstrap = useBootstrap()
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Settings · local control plane</p>
          <h1>Connections without credential custody.</h1>
        </div>
        <Badge tone="good">
          <LockKeyhole size={13} /> No hosted account
        </Badge>
      </header>
      <AgentSettings agents={bootstrap.data?.agents ?? []} />
      <BrowserSettings />
      <PlatformSettings />
      <DataSettings />
    </div>
  )
}

function AgentSettings({
  agents,
}: {
  agents: Array<{
    provider: "codex" | "claude"
    readiness: string
    version?: string | null
    detail?: string | null
  }>
}) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <span className="panel-icon">
          <Bot size={18} />
        </span>
        <div>
          <h2>Reasoning engines</h2>
          <p>Goalbar detects existing local Codex and Claude CLI sessions.</p>
        </div>
      </div>
      <div className="settings-list">
        {agents.map((agent) => (
          <div className="setting-row" key={agent.provider}>
            <div>
              <strong>{titleCase(agent.provider)}</strong>
              <small>{agent.version ?? agent.detail ?? "Version unavailable"}</small>
            </div>
            <Badge tone={agent.readiness === "ready" ? "good" : "warn"}>{titleCase(agent.readiness)}</Badge>
          </div>
        ))}
      </div>
      <p className="fine-print">
        Agents receive bounded content context and JSON Schema. They never receive social access tokens.
      </p>
    </section>
  )
}

function BrowserSettings() {
  return (
    <section className="panel browser-settings-callout">
      <div className="panel-heading">
        <span className="panel-icon">
          <Globe2 size={18} />
        </span>
        <div>
          <h2>Integrated browser</h2>
          <p>Use your existing website accounts locally, then explicitly preview and save evidence.</p>
        </div>
      </div>
      <div className="setting-row">
        <div>
          <strong>Recommended path</strong>
          <small>
            Sign in inside Goalbar. Website sessions stay in the desktop webview profile and never enter
            prompts or exports.
          </small>
        </div>
        <Link className="button button-primary button-small" to="/browser">
          Open Browser
        </Link>
      </div>
    </section>
  )
}

function PlatformSettings() {
  const bootstrap = useBootstrap()
  const queryClient = useQueryClient()
  const [form, setForm] = useState<BeginOAuthInput>({
    platform: "x",
    clientId: "",
    remoteAccountId: "",
    displayName: "",
    scopes: [...defaultScopes.x],
  })
  const [session, setSession] = useState<{ sessionId: string; authorizationUrl: string } | null>(null)
  const begin = useMutation({
    mutationFn: async () => {
      if (!isTauriRuntime())
        return beginOAuthResponseSchema.parse({
          sessionId: crypto.randomUUID(),
          authorizationUrl: "https://x.com/i/oauth2/authorize",
          redirectUri: "http://127.0.0.1:45678/oauth/callback",
          expiresAt: new Date(Date.now() + 180_000).toISOString(),
        })
      return invokeValidated(
        "begin_platform_oauth",
        { input: form },
        beginOAuthInputSchema,
        beginOAuthResponseSchema,
      )
    },
    onSuccess: (value) => setSession(value),
  })
  const complete = useMutation({
    mutationFn: async () => {
      if (!session) throw new Error("Start a connection first")
      if (!isTauriRuntime())
        return connectedAccountSchema.parse({
          id: crypto.randomUUID(),
          platform: form.platform,
          clientId: form.clientId,
          remoteAccountId: form.remoteAccountId,
          displayName: form.displayName,
          secretRef: "preview",
          scopes: form.scopes,
          capabilities: {
            authenticate: "supported",
            publish: "supported",
            readOwnContent: "approval_pending",
            metrics: "approval_pending",
            reply: "supported",
            directMessages: form.platform === "linkedin" ? "unsupported" : "supported",
          },
          status: "connected",
        })
      const status = await invokeValidated(
        "get_oauth_status",
        { sessionId: session.sessionId },
        z.string().uuid(),
        oauthStatusSchema,
      )
      if (status.status !== "code_received")
        throw new Error(status.error ?? "Finish consent in the system browser first")
      return invokeValidated(
        "complete_platform_oauth",
        { sessionId: session.sessionId },
        z.string().uuid(),
        connectedAccountSchema,
      )
    },
    onSuccess: async () => {
      setSession(null)
      await queryClient.invalidateQueries({ queryKey: queryKeys.bootstrap })
    },
  })
  const disconnect = useMutation({
    mutationFn: (accountId: string) =>
      isTauriRuntime()
        ? invokeValidated("disconnect_platform", { accountId }, z.string().uuid(), z.boolean())
        : Promise.resolve(true),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: queryKeys.bootstrap }),
  })
  const sync = useMutation({
    mutationFn: (accountId: string) =>
      isTauriRuntime()
        ? invokeValidated(
            "sync_platform_now",
            { accountId },
            z.string().uuid(),
            z.object({ items: z.array(z.unknown()), nextCursor: z.string().nullable().optional() }),
          )
        : Promise.resolve({ items: [], nextCursor: null }),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: queryKeys.bootstrap }),
  })
  const updatePlatform = (platform: BeginOAuthInput["platform"]) =>
    setForm((value) => ({ ...value, platform, scopes: [...defaultScopes[platform]] }))
  const scopeText = useMemo(() => form.scopes.join(" "), [form.scopes])

  return (
    <section className="panel">
      <div className="panel-heading">
        <span className="panel-icon">
          <Link2 size={18} />
        </span>
        <div>
          <h2>Official API connections</h2>
          <p>Optional advanced path for approved integrations and stable API actions.</p>
        </div>
      </div>
      {bootstrap.data?.accounts.length ? (
        <div className="account-list">
          {bootstrap.data.accounts.map((account) => (
            <article className="account-card" key={account.id}>
              <div>
                <Badge>{titleCase(account.platform)}</Badge>
                <h3>{account.displayName}</h3>
                <p>{account.remoteAccountId}</p>
              </div>
              <div className="capability-strip">
                <CapabilityBadge state={account.capabilities.publish} />
                <span>Publish</span>
                <CapabilityBadge state={account.capabilities.reply} />
                <span>Replies</span>
                <CapabilityBadge state={account.capabilities.directMessages} />
                <span>DMs</span>
              </div>
              <div className="account-actions">
                <Button
                  variant="secondary"
                  size="small"
                  onClick={() => sync.mutate(account.id)}
                  disabled={sync.isPending}
                >
                  Sync
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={`Disconnect ${account.displayName}`}
                  onClick={() => disconnect.mutate(account.id)}
                >
                  <Trash2 size={16} />
                </Button>
              </div>
            </article>
          ))}
        </div>
      ) : (
        <p className="muted-copy">
          No platform account is connected yet. You need your own approved platform application ID; no client
          secret is stored in the app.
        </p>
      )}
      <div className="connect-form">
        <div className="segmented" aria-label="Platform">
          {(["x", "reddit", "linkedin"] as const).map((platform) => (
            <button
              className={form.platform === platform ? "active" : ""}
              key={platform}
              onClick={() => updatePlatform(platform)}
            >
              {titleCase(platform)}
            </button>
          ))}
        </div>
        <div className="field-grid two">
          <label className="field">
            <span>Application / client ID</span>
            <Input
              value={form.clientId}
              onChange={(event) => setForm((value) => ({ ...value, clientId: event.target.value }))}
              placeholder="From the platform developer portal"
            />
          </label>
          <label className="field">
            <span>Account ID or actor URN</span>
            <Input
              value={form.remoteAccountId}
              onChange={(event) => setForm((value) => ({ ...value, remoteAccountId: event.target.value }))}
              placeholder={form.platform === "linkedin" ? "urn:li:person:…" : "Platform account ID"}
            />
          </label>
        </div>
        <label className="field">
          <span>Display name / handle</span>
          <Input
            value={form.displayName}
            onChange={(event) => setForm((value) => ({ ...value, displayName: event.target.value }))}
            placeholder="@founder or u/founder"
          />
        </label>
        <div className="scope-preview">
          <KeyRound size={16} />
          <span>
            <strong>Requested scopes</strong>
            <small>{scopeText}</small>
          </span>
        </div>
        {session ? (
          <div className="oauth-wait">
            <CheckCircle2 size={19} />
            <span>
              <strong>Consent window opened</strong>
              <small>
                Finish with {titleCase(form.platform)}, then return here. Session{" "}
                {session.sessionId.slice(0, 8)}…
              </small>
            </span>
            <Button size="small" onClick={() => complete.mutate()} disabled={complete.isPending}>
              {complete.isPending ? "Checking…" : "Finish connection"}
            </Button>
          </div>
        ) : (
          <Button
            onClick={() => begin.mutate()}
            disabled={begin.isPending || !form.clientId || !form.remoteAccountId || !form.displayName}
          >
            {begin.isPending ? "Opening browser…" : `Connect ${titleCase(form.platform)}`}
            <ExternalLink size={15} />
          </Button>
        )}
        {(begin.error || complete.error) && (
          <div className="inline-error">
            <strong>Connection not complete</strong>
            <span>{begin.error?.message ?? complete.error?.message}</span>
          </div>
        )}
      </div>
    </section>
  )
}

function DataSettings() {
  const [artifact, setArtifact] = useState<string | null>(null)
  const [browserConfirmation, setBrowserConfirmation] = useState("")
  const exportData = useMutation({
    mutationFn: () =>
      isTauriRuntime()
        ? invokeOutput("export_local_data", {}, dataArtifactSchema)
        : Promise.resolve(
            dataArtifactSchema.parse({
              path: "/preview/export.json",
              kind: "json_export",
              createdAt: new Date().toISOString(),
              includesSecrets: false,
            }),
          ),
    onSuccess: (value) => setArtifact(value.path),
  })
  const backup = useMutation({
    mutationFn: () =>
      isTauriRuntime()
        ? invokeOutput("backup_local_database", {}, dataArtifactSchema)
        : Promise.resolve(
            dataArtifactSchema.parse({
              path: "/preview/backup.sqlite",
              kind: "sqlite_backup",
              createdAt: new Date().toISOString(),
              includesSecrets: false,
            }),
          ),
    onSuccess: (value) => setArtifact(value.path),
  })
  const clearBrowserData = useMutation({
    mutationFn: () => {
      const input = { confirmation: browserConfirmation }
      return isTauriRuntime()
        ? invokeValidated("clear_browser_data", { input }, clearBrowserDataInputSchema, z.boolean())
        : Promise.resolve(clearBrowserDataInputSchema.parse(input)).then(() => true)
    },
    onSuccess: () => setBrowserConfirmation(""),
  })
  return (
    <section className="panel">
      <div className="panel-heading">
        <span className="panel-icon">
          <LockKeyhole size={18} />
        </span>
        <div>
          <h2>Local data</h2>
          <p>SQLite stores product memory; the OS keyring stores tokens.</p>
        </div>
      </div>
      <div className="settings-list">
        <div className="setting-row">
          <div>
            <strong>Portable JSON export</strong>
            <small>Founder memory, experiments, and learnings. Credentials are always excluded.</small>
          </div>
          <Button
            variant="secondary"
            size="small"
            onClick={() => exportData.mutate()}
            disabled={exportData.isPending}
          >
            Export JSON
          </Button>
        </div>
        <div className="setting-row">
          <div>
            <strong>SQLite backup</strong>
            <small>Consistent local database snapshot. OS-keyring credentials are not included.</small>
          </div>
          <Button
            variant="secondary"
            size="small"
            onClick={() => backup.mutate()}
            disabled={backup.isPending}
          >
            Create backup
          </Button>
        </div>
        <div className="setting-row browser-data-reset">
          <div>
            <strong>Clear integrated browser data</strong>
            <small>Removes local website cookies, storage, and cache. Imported history is not removed.</small>
          </div>
          <div className="confirmation-action">
            <Input
              aria-label="Browser data confirmation"
              value={browserConfirmation}
              onChange={(event) => setBrowserConfirmation(event.target.value)}
              placeholder="Type CLEAR BROWSER DATA"
            />
            <Button
              variant="danger"
              size="small"
              disabled={browserConfirmation !== "CLEAR BROWSER DATA" || clearBrowserData.isPending}
              onClick={() => clearBrowserData.mutate()}
            >
              Clear
            </Button>
          </div>
        </div>
      </div>
      {artifact && (
        <div className="scope-preview">
          <CheckCircle2 size={16} />
          <span>
            <strong>Local artifact created</strong>
            <small>{artifact}</small>
          </span>
        </div>
      )}
      {clearBrowserData.isSuccess && (
        <p className="success-note">
          <CheckCircle2 size={14} /> Integrated browser data cleared.
        </p>
      )}
      {clearBrowserData.error && (
        <div className="inline-error">
          <strong>Browser data not cleared</strong>
          <span>{clearBrowserData.error.message}</span>
        </div>
      )}
    </section>
  )
}
