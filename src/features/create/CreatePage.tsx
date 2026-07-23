import { zodResolver } from "@hookform/resolvers/zod"
import { useMutation } from "@tanstack/react-query"
import { Check, Copy, FlaskConical, Sparkles } from "lucide-react"
import { useState } from "react"
import { useForm } from "react-hook-form"
import { Link } from "react-router-dom"

import { useBootstrap } from "@/app/bootstrap"
import { EmptyState } from "@/components/EmptyState"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { titleCase } from "@/lib/utils"
import {
  approvalSchema,
  approveVariantInputSchema,
  contentIdeaSchema,
  generateContentInputSchema,
  generateContentResponseSchema,
  publishVariantInputSchema,
  remoteContentSchema,
  type ContentIdea,
  type StoredContentVariant,
} from "@/schemas/content"

export function CreatePage() {
  const bootstrap = useBootstrap()
  const [variants, setVariants] = useState<StoredContentVariant[]>([])
  const [approved, setApproved] = useState<Map<string, string>>(new Map())
  const [published, setPublished] = useState<Set<string>>(new Set())
  const [redditDestination, setRedditDestination] = useState("")
  const form = useForm<ContentIdea>({
    resolver: zodResolver(contentIdeaSchema),
    defaultValues: {
      title: "",
      insight: "",
      hypothesis: "",
      successMetric: "Qualified replies from the target ICP within 7 days",
    },
  })
  const provider = bootstrap.data?.agents.find((agent) => agent.readiness === "ready")?.provider ?? "codex"
  const generate = useMutation({
    mutationFn: async (idea: ContentIdea) => {
      const input = { provider, idea }
      if (!isTauriRuntime())
        return generateContentResponseSchema.parse({
          ideaId: crypto.randomUUID(),
          variants: previewVariants(idea.insight),
        })
      return invokeValidated(
        "generate_content_variants",
        { input },
        generateContentInputSchema,
        generateContentResponseSchema,
      )
    },
    onSuccess: (response) => setVariants(response.variants),
  })
  const approve = useMutation({
    mutationFn: async (variant: StoredContentVariant) => {
      const input = { variantId: variant.id, body: variant.body }
      if (!isTauriRuntime())
        return approvalSchema.parse({
          id: crypto.randomUUID(),
          subjectType: "content_variant",
          subjectId: variant.id,
          payloadHash: "preview",
          idempotencyKey: crypto.randomUUID(),
          approvedAt: new Date().toISOString(),
        })
      return invokeValidated("approve_variant", { input }, approveVariantInputSchema, approvalSchema)
    },
    onSuccess: (approval, variant) => setApproved((current) => new Map(current).set(variant.id, approval.id)),
  })
  const publish = useMutation({
    mutationFn: async (variant: StoredContentVariant) => {
      const account = bootstrap.data?.accounts.find((candidate) => candidate.platform === variant.platform)
      const approvalId = approved.get(variant.id)
      if (!account || !approvalId) throw new Error("Connect this platform and approve the exact text first")
      const input = {
        accountId: account.id,
        approvalId,
        variantId: variant.id,
        body: variant.body,
        title: variant.platform === "reddit" ? form.getValues("title") : undefined,
        destination: variant.platform === "reddit" ? redditDestination : undefined,
      }
      if (!isTauriRuntime())
        return remoteContentSchema.parse({
          platform: variant.platform,
          remoteId: crypto.randomUUID(),
          body: variant.body,
        })
      return invokeValidated("publish_variant", { input }, publishVariantInputSchema, remoteContentSchema)
    },
    onSuccess: (_, variant) => setPublished((current) => new Set(current).add(variant.id)),
  })

  if (bootstrap.isPending)
    return (
      <div className="page-state">
        <span className="pulse-dot" />
        <h1>Loading local memory…</h1>
      </div>
    )
  if (!bootstrap.data?.founder)
    return (
      <EmptyState
        eyebrow="Founder baseline needed"
        title="Create from a point of view, not a blank prompt."
        body="Complete the private onboarding once so every draft has your expertise, boundaries, and intended customer in context."
        action={
          <Link to="/onboarding">
            <Button>Start onboarding</Button>
          </Link>
        }
      />
    )

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Create · one idea, three native expressions</p>
          <h1>Design the next experiment.</h1>
        </div>
        <Badge tone="good">
          <FlaskConical size={13} /> Human approval required
        </Badge>
      </header>
      <div className="create-layout">
        <form
          className="panel form-panel"
          onSubmit={(event) => void form.handleSubmit((value) => generate.mutate(value))(event)}
        >
          <div className="panel-heading">
            <span className="panel-icon">
              <Sparkles size={17} />
            </span>
            <div>
              <h2>Source insight</h2>
              <p>Give the agent something true and worth testing.</p>
            </div>
          </div>
          <Field label="Working title" error={form.formState.errors.title?.message}>
            <Input {...form.register("title")} placeholder="The learning-loop problem" />
          </Field>
          <Field label="The insight" error={form.formState.errors.insight?.message}>
            <Textarea
              rows={7}
              {...form.register("insight")}
              placeholder="Most solo founders do not have a lead problem…"
            />
          </Field>
          <Field label="Experiment hypothesis" error={form.formState.errors.hypothesis?.message}>
            <Textarea rows={3} {...form.register("hypothesis")} placeholder="I believe this framing will…" />
          </Field>
          <Field label="Success signal" error={form.formState.errors.successMetric?.message}>
            <Input {...form.register("successMetric")} />
          </Field>
          {generate.isError && (
            <div className="inline-error">
              <strong>Generation failed</strong>
              <span>{generate.error.message}</span>
            </div>
          )}
          <Button type="submit" disabled={generate.isPending}>
            {generate.isPending ? `Asking ${titleCase(provider)}…` : `Generate with ${titleCase(provider)}`}
          </Button>
        </form>
        <section className="variant-column" aria-live="polite">
          {!variants.length ? (
            <div className="draft-placeholder">
              <Copy size={25} />
              <h2>Your native drafts appear here.</h2>
              <p>
                The app sends bounded founder and idea context to {titleCase(provider)}. It never sends
                platform tokens.
              </p>
            </div>
          ) : (
            variants.map((variant) => {
              const account = bootstrap.data?.accounts.find(
                (candidate) => candidate.platform === variant.platform,
              )
              return (
                <article className="variant-card" key={variant.id}>
                  <div className="variant-header">
                    <Badge>{titleCase(variant.platform)}</Badge>
                    <span>Revision {variant.revision}</span>
                  </div>
                  {variant.platform === "reddit" && (
                    <label className="field variant-destination">
                      <span>Destination subreddit</span>
                      <Input
                        value={redditDestination}
                        onChange={(event) => setRedditDestination(event.target.value)}
                        placeholder="SaaS"
                      />
                    </label>
                  )}
                  <Textarea
                    value={variant.body}
                    rows={variant.platform === "x" ? 5 : 9}
                    onChange={(event) => {
                      setVariants((items) =>
                        items.map((item) =>
                          item.id === variant.id ? { ...item, body: event.target.value } : item,
                        ),
                      )
                      setApproved((current) => {
                        const next = new Map(current)
                        next.delete(variant.id)
                        return next
                      })
                    }}
                  />
                  <div className="variant-footer">
                    <small>
                      {account
                        ? `Ready for ${account.displayName}`
                        : `Connect ${titleCase(variant.platform)} before publishing.`}
                    </small>
                    {published.has(variant.id) ? (
                      <Badge tone="good">
                        <Check size={12} /> Published
                      </Badge>
                    ) : approved.has(variant.id) ? (
                      <Button
                        size="small"
                        disabled={
                          !account ||
                          publish.isPending ||
                          (variant.platform === "reddit" && !redditDestination)
                        }
                        onClick={() => publish.mutate(variant)}
                      >
                        {publish.isPending ? "Publishing…" : "Publish approved text"}
                      </Button>
                    ) : (
                      <Button
                        size="small"
                        disabled={approve.isPending}
                        onClick={() => approve.mutate(variant)}
                      >
                        Approve exact text
                      </Button>
                    )}
                  </div>
                </article>
              )
            })
          )}
        </section>
      </div>
    </div>
  )
}

function previewVariants(insight: string): StoredContentVariant[] {
  return (["x", "reddit", "linkedin"] as const).map((platform) => ({
    id: crypto.randomUUID(),
    platform,
    revision: 1,
    status: "draft",
    body:
      platform === "x"
        ? `${insight}\n\nThe real advantage is how fast you learn.`
        : platform === "reddit"
          ? `What I keep seeing with solo founders\n\n${insight}\n\nHow are you measuring the learning loop?`
          : `${insight}\n\nI used to mistake consistency for progress. The useful question is whether each post changes what you know about the people you serve.`,
  }))
}

function Field({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {error && <em>{error}</em>}
    </label>
  )
}
