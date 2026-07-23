import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { BrainCircuit, Check, Pencil, Sparkles, X } from "lucide-react"
import { useState } from "react"
import { z } from "zod"

import { useBootstrap } from "@/app/bootstrap"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { queryKeys } from "@/lib/query-keys"
import { invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  founderInputSchema,
  founderProfileSchema,
  updateFounderInputSchema,
  type FounderInput,
  type FounderProfile,
} from "@/schemas/founder"
import {
  acceptIcpInputSchema,
  icpHypothesesSchema,
  reviseIcpInputSchema,
  saveVoiceInputSchema,
  storedIcpHypothesesSchema,
  type icpHypothesisSchema,
} from "@/schemas/memory"
import { historyOverviewSchema } from "@/schemas/history"

type IcpHypothesis = z.infer<typeof icpHypothesisSchema>

function founderInputFromProfile(founder: FounderProfile): FounderInput {
  return {
    name: founder.name,
    productName: founder.productName,
    websiteUrl: founder.websiteUrl ?? "",
    offer: founder.offer,
    idealCustomer: founder.idealCustomer,
    expertise: founder.expertise,
    goals: founder.goals,
    boundaries: founder.boundaries,
  }
}

function lines(value: string) {
  return value.split("\n")
}

function commaSeparated(value: string) {
  return value.split(",")
}

function normalizedItems(values: string[]) {
  return values.map((item) => item.trim()).filter(Boolean)
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Something went wrong"
}

export function MemoryPage() {
  const { data } = useBootstrap()
  const queryClient = useQueryClient()
  const [example, setExample] = useState("")
  const [traits, setTraits] = useState("direct, curious, practical")
  const [icp, setIcp] = useState<z.infer<typeof icpHypothesesSchema> | null>(null)
  const [isEditingBaseline, setIsEditingBaseline] = useState(false)
  const [founderDraft, setFounderDraft] = useState<FounderInput | null>(null)
  const [baselineValidationError, setBaselineValidationError] = useState<string | null>(null)
  const [editingIcp, setEditingIcp] = useState<{
    sourceId: string
    hypothesis: IcpHypothesis
  } | null>(null)
  const [icpValidationError, setIcpValidationError] = useState<string | null>(null)
  const codex = data?.agents.find((agent) => agent.provider === "codex")
  const isCodexReady = codex?.readiness === "ready"
  const storedIcp = useQuery({
    queryKey: queryKeys.icp,
    enabled: Boolean(data?.founder),
    queryFn: async () => {
      if (!isTauriRuntime()) return []
      return invokeValidated("list_icp_hypotheses", {}, z.object({}), storedIcpHypothesesSchema)
    },
  })
  const history = useQuery({
    queryKey: queryKeys.history,
    queryFn: async () => {
      if (!isTauriRuntime())
        return historyOverviewSchema.parse({
          schemaVersion: 1,
          sourceCount: 0,
          itemCount: 0,
          platforms: [],
        })
      return invokeValidated("get_history_overview", {}, z.object({}).strict(), historyOverviewSchema)
    },
  })
  const saveVoice = useMutation({
    mutationFn: async () => {
      if (!data?.founder) throw new Error("Complete founder onboarding first")
      const input = {
        founderId: data.founder.id,
        voice: {
          traits: traits
            .split(",")
            .map((item) => item.trim())
            .filter(Boolean),
          doRules: ["Use specific experience and honest uncertainty"],
          dontRules: ["Do not invent claims or use aggressive selling"],
          example,
        },
      }
      if (!isTauriRuntime()) return crypto.randomUUID()
      return invokeValidated("save_voice_profile", { input }, saveVoiceInputSchema, z.string().uuid())
    },
  })
  const updateFounder = useMutation({
    mutationFn: async (profile: FounderInput) => {
      if (!data?.founder) throw new Error("Complete founder onboarding first")
      const input = { founderId: data.founder.id, profile }
      if (!isTauriRuntime()) {
        return founderProfileSchema.parse({
          ...data.founder,
          ...profile,
          websiteUrl: profile.websiteUrl || null,
          updatedAt: new Date().toISOString(),
        })
      }
      return invokeValidated(
        "update_founder_profile",
        { input },
        updateFounderInputSchema,
        founderProfileSchema,
      )
    },
    onSuccess: async () => {
      setIsEditingBaseline(false)
      setFounderDraft(null)
      setBaselineValidationError(null)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.bootstrap }),
        queryClient.invalidateQueries({ queryKey: queryKeys.icp }),
      ])
    },
  })
  const discoverIcp = useMutation({
    mutationFn: async () => {
      const input = { provider: "codex" as const }
      if (!isTauriRuntime())
        return icpHypothesesSchema.parse({
          hypotheses: [
            {
              role: "Technical solo SaaS founder",
              situation: "Has early product traction but inconsistent distribution",
              urgentProblem: "Content creates reach without enough customer learning",
              currentWorkaround: "Posting ad hoc and checking vanity metrics",
              desiredOutcome: "A repeatable loop from insight to qualified conversation",
              objections: ["Another tool could create more work"],
              language: ["learning loop", "qualified conversation"],
              confidence: 0.45,
            },
          ],
        })
      return invokeValidated(
        "generate_icp_hypotheses",
        { input },
        z.object({ provider: z.literal("codex") }),
        icpHypothesesSchema,
      )
    },
    onSuccess: async (value) => {
      setIcp(value)
      await queryClient.invalidateQueries({ queryKey: queryKeys.icp })
    },
  })
  const reviseIcp = useMutation({
    mutationFn: async (input: z.infer<typeof reviseIcpInputSchema>) => {
      if (!isTauriRuntime()) return crypto.randomUUID()
      return invokeValidated("revise_icp_hypothesis", { input }, reviseIcpInputSchema, z.string().uuid())
    },
    onSuccess: async () => {
      setEditingIcp(null)
      setIcpValidationError(null)
      await queryClient.invalidateQueries({ queryKey: queryKeys.icp })
    },
  })
  const acceptIcp = useMutation({
    mutationFn: async (hypothesisId: string) => {
      const input = { hypothesisId }
      if (!isTauriRuntime()) return null
      return invokeValidated("accept_icp_hypothesis", { input }, acceptIcpInputSchema, z.null())
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.icp })
    },
  })
  const startBaselineEdit = () => {
    if (!data?.founder) return
    updateFounder.reset()
    setBaselineValidationError(null)
    setFounderDraft(founderInputFromProfile(data.founder))
    setIsEditingBaseline(true)
  }
  const updateFounderDraft = <Key extends keyof FounderInput>(key: Key, value: FounderInput[Key]) => {
    setFounderDraft((current) => (current ? { ...current, [key]: value } : current))
  }
  const submitFounderUpdate = () => {
    if (!founderDraft) return
    const parsed = founderInputSchema.safeParse({
      ...founderDraft,
      goals: normalizedItems(founderDraft.goals),
      boundaries: normalizedItems(founderDraft.boundaries),
    })
    if (!parsed.success) {
      setBaselineValidationError(parsed.error.issues[0]?.message ?? "Review the founder baseline")
      return
    }
    setBaselineValidationError(null)
    updateFounder.mutate(parsed.data)
  }
  const startIcpEdit = (sourceId: string, hypothesis: IcpHypothesis) => {
    reviseIcp.reset()
    setIcpValidationError(null)
    setEditingIcp({ sourceId, hypothesis })
  }
  const updateIcpDraft = <Key extends keyof IcpHypothesis>(key: Key, value: IcpHypothesis[Key]) => {
    setEditingIcp((current) =>
      current
        ? {
            ...current,
            hypothesis: { ...current.hypothesis, [key]: value },
          }
        : current,
    )
  }
  const submitIcpRevision = () => {
    if (!editingIcp) return
    const input = reviseIcpInputSchema.safeParse({
      hypothesisId: editingIcp.sourceId,
      hypothesis: {
        ...editingIcp.hypothesis,
        objections: normalizedItems(editingIcp.hypothesis.objections),
        language: normalizedItems(editingIcp.hypothesis.language),
      },
    })
    if (!input.success) {
      setIcpValidationError(input.error.issues[0]?.message ?? "Review the ICP revision")
      return
    }
    setIcpValidationError(null)
    reviseIcp.mutate(input.data)
  }
  const displayedIcp =
    storedIcp.data && storedIcp.data.length > 0
      ? storedIcp.data
      : (icp?.hypotheses.map((hypothesis) => ({
          ...hypothesis,
          id: null,
          version: 1,
          parentId: null,
          status: "proposed" as const,
        })) ?? [])

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Memory · editable and local</p>
          <h1>Your founder point of view.</h1>
        </div>
        <Badge tone="good">SQLite on this machine</Badge>
      </header>
      <div className="memory-grid">
        <section className="panel history-overview-card">
          <div className="panel-heading">
            <span className="panel-icon">
              <BrainCircuit size={17} />
            </span>
            <div>
              <h2>Historical evidence</h2>
              <p>Normalized from official archives and your explicit browser captures.</p>
            </div>
          </div>
          <div className="history-overview-stat">
            <strong>{history.data?.itemCount ?? 0}</strong>
            <span>local activity items across {history.data?.sourceCount ?? 0} sources</span>
          </div>
          <div className="category-grid">
            {history.data?.platforms.map((platform) => (
              <span key={platform.platform}>
                {platform.platform} <strong>{platform.itemCount}</strong>
              </span>
            ))}
          </div>
          {!history.data?.itemCount && (
            <p className="muted-copy">
              Import an account archive or save an explicit browser capture to teach Goalbar from evidence.
            </p>
          )}
        </section>
        <section className="panel">
          <div className="memory-section-heading">
            <div>
              <p className="eyebrow">Founder baseline</p>
              <h2>{data?.founder?.productName ?? "Not configured"}</h2>
            </div>
            {data?.founder && !isEditingBaseline && (
              <Button
                size="small"
                variant="ghost"
                onClick={startBaselineEdit}
                aria-label="Edit founder baseline"
              >
                <Pencil size={13} /> Edit
              </Button>
            )}
          </div>
          {isEditingBaseline && founderDraft ? (
            <form
              className="memory-editor"
              onSubmit={(event) => {
                event.preventDefault()
                submitFounderUpdate()
              }}
            >
              <div className="memory-form-grid">
                <label className="field">
                  <span>Founder name</span>
                  <Input
                    value={founderDraft.name}
                    onChange={(event) => updateFounderDraft("name", event.target.value)}
                  />
                </label>
                <label className="field">
                  <span>Product or company</span>
                  <Input
                    value={founderDraft.productName}
                    onChange={(event) => updateFounderDraft("productName", event.target.value)}
                  />
                </label>
              </div>
              <label className="field">
                <span>Website</span>
                <Input
                  type="url"
                  value={founderDraft.websiteUrl}
                  onChange={(event) => updateFounderDraft("websiteUrl", event.target.value)}
                  placeholder="https://…"
                />
              </label>
              <label className="field">
                <span>What you offer</span>
                <Textarea
                  rows={3}
                  value={founderDraft.offer}
                  onChange={(event) => updateFounderDraft("offer", event.target.value)}
                />
              </label>
              <label className="field">
                <span>Current ideal customer</span>
                <Textarea
                  rows={3}
                  value={founderDraft.idealCustomer}
                  onChange={(event) => updateFounderDraft("idealCustomer", event.target.value)}
                />
              </label>
              <label className="field">
                <span>Expertise and point of view</span>
                <Textarea
                  rows={3}
                  value={founderDraft.expertise}
                  onChange={(event) => updateFounderDraft("expertise", event.target.value)}
                />
              </label>
              <div className="memory-form-grid">
                <label className="field">
                  <span>Goals, one per line</span>
                  <Textarea
                    rows={3}
                    value={founderDraft.goals.join("\n")}
                    onChange={(event) => updateFounderDraft("goals", lines(event.target.value))}
                  />
                </label>
                <label className="field">
                  <span>Boundaries, one per line</span>
                  <Textarea
                    rows={3}
                    value={founderDraft.boundaries.join("\n")}
                    onChange={(event) => updateFounderDraft("boundaries", lines(event.target.value))}
                  />
                </label>
              </div>
              <p className="memory-local-note">
                This updates the same local profile, so its evidence and ICP versions stay attached.
              </p>
              {(baselineValidationError || updateFounder.isError) && (
                <p className="form-error" role="alert">
                  {baselineValidationError ?? errorMessage(updateFounder.error)}
                </p>
              )}
              <div className="memory-actions">
                <Button size="small" type="submit" disabled={updateFounder.isPending}>
                  {updateFounder.isPending ? "Saving…" : "Save baseline"}
                </Button>
                <Button
                  size="small"
                  type="button"
                  variant="ghost"
                  onClick={() => {
                    setIsEditingBaseline(false)
                    setFounderDraft(null)
                    setBaselineValidationError(null)
                  }}
                >
                  <X size={13} /> Cancel
                </Button>
              </div>
            </form>
          ) : (
            <>
              <p className="muted-copy">
                {data?.founder?.offer ||
                  data?.founder?.websiteUrl ||
                  "Complete onboarding to establish your starting context."}
              </p>
              {data?.founder && (
                <dl className="definition-list">
                  <div>
                    <dt>Founder</dt>
                    <dd>{data.founder.name}</dd>
                  </div>
                  <div>
                    <dt>Ideal customer</dt>
                    <dd>{data.founder.idealCustomer || "Not added yet"}</dd>
                  </div>
                  {data.founder.expertise && (
                    <div>
                      <dt>Expertise</dt>
                      <dd>{data.founder.expertise}</dd>
                    </div>
                  )}
                  <div>
                    <dt>Goal</dt>
                    <dd>{data.founder.goals[0] ?? "Not set"}</dd>
                  </div>
                </dl>
              )}
            </>
          )}
        </section>
        <section className="panel">
          <div className="panel-heading">
            <span className="panel-icon">
              <BrainCircuit size={17} />
            </span>
            <div>
              <h2>Founder voice</h2>
              <p>Approve examples and explicit boundaries.</p>
            </div>
          </div>
          <label className="field">
            <span>Tone traits, separated by commas</span>
            <Input value={traits} onChange={(event) => setTraits(event.target.value)} />
          </label>
          <label className="field memory-field">
            <span>A real example that sounds like you</span>
            <Textarea
              rows={5}
              value={example}
              onChange={(event) => setExample(event.target.value)}
              placeholder="Paste something you wrote and would happily publish again…"
            />
          </label>
          <Button
            size="small"
            onClick={() => saveVoice.mutate()}
            disabled={example.length < 10 || saveVoice.isPending}
          >
            {saveVoice.isSuccess ? (
              <>
                <Check size={14} /> Voice saved
              </>
            ) : (
              "Save approved voice"
            )}
          </Button>
        </section>
        <section className="panel">
          <div className="memory-section-heading memory-section-heading-top">
            <div className="panel-heading">
              <span className="panel-icon">
                <Sparkles size={17} />
              </span>
              <div>
                <h2>ICP hypotheses</h2>
                <p>Editable versions, grounded in evidence instead of silent truth.</p>
              </div>
            </div>
            <Badge tone={isCodexReady ? "good" : "warn"}>
              {isCodexReady ? "Your Codex CLI" : "Codex setup needed"}
            </Badge>
          </div>
          <p className="memory-local-note">
            Manual edits become reviewable versions. On refresh, Codex sees the active version, your latest
            baseline, and approved local evidence. No model API key is used.
          </p>
          {displayedIcp.length > 0 ? (
            <div className="hypothesis-list">
              {displayedIcp.map((hypothesis) => (
                <article key={hypothesis.id ?? `${hypothesis.role}-${hypothesis.situation}`}>
                  {hypothesis.id && editingIcp?.sourceId === hypothesis.id ? (
                    <form
                      className="memory-editor"
                      onSubmit={(event) => {
                        event.preventDefault()
                        submitIcpRevision()
                      }}
                    >
                      <div className="memory-revision-heading">
                        <Badge tone="warn">Editing from v{hypothesis.version}</Badge>
                        <span>The original stays in local revision history.</span>
                      </div>
                      <label className="field">
                        <span>Who this customer is</span>
                        <Input
                          value={editingIcp.hypothesis.role}
                          onChange={(event) => updateIcpDraft("role", event.target.value)}
                        />
                      </label>
                      <label className="field">
                        <span>Situation</span>
                        <Textarea
                          rows={2}
                          value={editingIcp.hypothesis.situation}
                          onChange={(event) => updateIcpDraft("situation", event.target.value)}
                        />
                      </label>
                      <label className="field">
                        <span>Urgent problem</span>
                        <Textarea
                          rows={2}
                          value={editingIcp.hypothesis.urgentProblem}
                          onChange={(event) => updateIcpDraft("urgentProblem", event.target.value)}
                        />
                      </label>
                      <label className="field">
                        <span>Current workaround</span>
                        <Textarea
                          rows={2}
                          value={editingIcp.hypothesis.currentWorkaround}
                          onChange={(event) => updateIcpDraft("currentWorkaround", event.target.value)}
                        />
                      </label>
                      <label className="field">
                        <span>Desired outcome</span>
                        <Textarea
                          rows={2}
                          value={editingIcp.hypothesis.desiredOutcome}
                          onChange={(event) => updateIcpDraft("desiredOutcome", event.target.value)}
                        />
                      </label>
                      <div className="memory-form-grid">
                        <label className="field">
                          <span>Objections, separated by commas</span>
                          <Textarea
                            rows={2}
                            value={editingIcp.hypothesis.objections.join(", ")}
                            onChange={(event) =>
                              updateIcpDraft("objections", commaSeparated(event.target.value))
                            }
                          />
                        </label>
                        <label className="field">
                          <span>Customer language, separated by commas</span>
                          <Textarea
                            rows={2}
                            value={editingIcp.hypothesis.language.join(", ")}
                            onChange={(event) =>
                              updateIcpDraft("language", commaSeparated(event.target.value))
                            }
                          />
                        </label>
                      </div>
                      <label className="field memory-confidence-field">
                        <span>Confidence</span>
                        <Input
                          type="number"
                          min={0}
                          max={100}
                          step={1}
                          value={Math.round(editingIcp.hypothesis.confidence * 100)}
                          onChange={(event) => {
                            const percentage = Number(event.target.value)
                            if (Number.isFinite(percentage)) {
                              updateIcpDraft("confidence", Math.min(100, Math.max(0, percentage)) / 100)
                            }
                          }}
                        />
                        <small>Percent, based on accepted evidence—not certainty.</small>
                      </label>
                      {(icpValidationError || reviseIcp.isError) && (
                        <p className="form-error" role="alert">
                          {icpValidationError ?? errorMessage(reviseIcp.error)}
                        </p>
                      )}
                      <div className="memory-actions">
                        <Button size="small" type="submit" disabled={reviseIcp.isPending}>
                          {reviseIcp.isPending ? "Saving revision…" : "Save as proposed version"}
                        </Button>
                        <Button
                          size="small"
                          type="button"
                          variant="ghost"
                          onClick={() => {
                            setEditingIcp(null)
                            setIcpValidationError(null)
                          }}
                        >
                          <X size={13} /> Cancel
                        </Button>
                      </div>
                    </form>
                  ) : (
                    <>
                      <div className="memory-revision-heading">
                        <Badge tone={hypothesis.status === "active" ? "good" : "warn"}>
                          v{hypothesis.version} ·{" "}
                          {hypothesis.status === "active" ? "Active · " : "Proposed · "}
                          {Math.round(hypothesis.confidence * 100)}% confidence
                        </Badge>
                        {hypothesis.parentId && <span>Revised from an earlier version</span>}
                      </div>
                      <h3>{hypothesis.role}</h3>
                      <p>{hypothesis.urgentProblem}</p>
                      <small>{hypothesis.situation}</small>
                      <dl className="icp-detail-list">
                        <div>
                          <dt>Workaround</dt>
                          <dd>{hypothesis.currentWorkaround}</dd>
                        </div>
                        <div>
                          <dt>Outcome</dt>
                          <dd>{hypothesis.desiredOutcome}</dd>
                        </div>
                      </dl>
                      {hypothesis.id && (
                        <div className="memory-actions">
                          <Button
                            size="small"
                            variant="ghost"
                            onClick={() => startIcpEdit(hypothesis.id, hypothesis)}
                          >
                            <Pencil size={13} /> Edit into a new version
                          </Button>
                          {hypothesis.status === "proposed" && (
                            <Button
                              size="small"
                              variant="secondary"
                              onClick={() => acceptIcp.mutate(hypothesis.id)}
                              disabled={acceptIcp.isPending}
                            >
                              Accept this version
                            </Button>
                          )}
                        </div>
                      )}
                    </>
                  )}
                </article>
              ))}
            </div>
          ) : (
            <p className="muted-copy">
              Ask your installed Codex CLI to propose an initial segment, urgent problem, current workaround,
              desired outcome, language, and objections from your approved baseline.
            </p>
          )}
          {acceptIcp.isError && (
            <p className="form-error" role="alert">
              {errorMessage(acceptIcp.error)}
            </p>
          )}
          {discoverIcp.isError && (
            <p className="form-error" role="alert">
              {errorMessage(discoverIcp.error)}
            </p>
          )}
          <Button
            className="memory-button"
            size="small"
            variant="secondary"
            onClick={() => discoverIcp.mutate()}
            disabled={!data?.founder || !isCodexReady || discoverIcp.isPending}
          >
            {discoverIcp.isPending
              ? "Codex is reviewing local memory…"
              : displayedIcp.length > 0
                ? "Refresh with Codex"
                : "Discover with Codex"}
          </Button>
          {!isCodexReady && (
            <p className="memory-local-note">
              Install or sign in to Codex CLI to enable adaptive ICP refreshes.
            </p>
          )}
        </section>
      </div>
    </div>
  )
}
