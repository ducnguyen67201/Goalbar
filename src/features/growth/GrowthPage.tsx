import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Activity, CheckCircle2, ClipboardCheck, FlaskConical, Gauge, Plus, ShieldCheck } from "lucide-react"
import { useState } from "react"
import { z } from "zod"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { titleCase } from "@/lib/utils"
import { approvalSchema } from "@/schemas/content"
import {
  approveGrowthActionInputSchema,
  growthActionSchema,
  growthLoopOverviewSchema,
  growthScoreSchema,
  proposeGrowthActionInputSchema,
  recordGrowthActionExecutionInputSchema,
  recordGrowthActionMetricInputSchema,
  recordGrowthLearningInputSchema,
  reviseGrowthActionInputSchema,
  trackedGrowthLearningSchema,
  weeklyLearningSchema,
  type GrowthAction,
  type GrowthLoopOverview,
  type ProposeGrowthActionInput,
} from "@/schemas/growth"

const componentLabels = [
  "attentionQuality",
  "conversationQuality",
  "relationshipGrowth",
  "consistency",
  "learningVelocity",
] as const

const initialDraft = {
  kind: "comment" as const,
  platform: "x" as const,
  title: "",
  rationale: "",
  targetUrl: "",
  exactPayload: "",
  hypothesis: "",
  successMetric: "One qualified reply from the active ICP within 7 days",
  evaluationWindowDays: 7,
}

export function GrowthPage() {
  const queryClient = useQueryClient()
  const [learning, setLearning] = useState<z.infer<typeof weeklyLearningSchema> | null>(null)
  const [draft, setDraft] = useState(initialDraft)
  const growth = useQuery({
    queryKey: queryKeys.growth,
    queryFn: () =>
      isTauriRuntime()
        ? invokeOutput("get_growth_overview", {}, growthScoreSchema)
        : Promise.resolve(
            growthScoreSchema.parse({
              formulaVersion: 1,
              score: 0,
              confidence: 0,
              components: {},
              missing: componentLabels,
            }),
          ),
  })
  const loop = useQuery({
    queryKey: queryKeys.growthLoop,
    queryFn: () =>
      isTauriRuntime()
        ? invokeOutput("get_growth_loop_overview", {}, growthLoopOverviewSchema)
        : Promise.resolve(previewGrowthLoop()),
  })
  const refreshLoop = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.growthLoop }),
      queryClient.invalidateQueries({ queryKey: queryKeys.growth }),
      queryClient.invalidateQueries({ queryKey: queryKeys.bootstrap }),
    ])
  }
  const propose = useMutation({
    mutationFn: async () => {
      const input = proposeGrowthActionInputSchema.parse({
        icpHypothesisId: loop.data?.activeIcp?.id,
        kind: draft.kind,
        platform: draft.platform,
        title: draft.title,
        rationale: draft.rationale,
        targetUrl: draft.targetUrl || undefined,
        exactPayload: draft.exactPayload,
        hypothesis: draft.hypothesis,
        successMetric: draft.successMetric,
        evaluationWindowDays: draft.evaluationWindowDays,
      })
      if (!isTauriRuntime()) return previewAction(input)
      return invokeValidated(
        "propose_growth_action",
        { input },
        proposeGrowthActionInputSchema,
        growthActionSchema,
      )
    },
    onSuccess: async (action) => {
      if (isTauriRuntime()) {
        await refreshLoop()
      } else {
        queryClient.setQueryData<GrowthLoopOverview>(queryKeys.growthLoop, (current) => ({
          ...(current ?? previewGrowthLoop()),
          actions: [action, ...(current?.actions ?? [])],
          totals: {
            ...(current?.totals ?? previewGrowthLoop().totals),
            proposed: (current?.totals.proposed ?? 0) + 1,
          },
        }))
      }
      setDraft(initialDraft)
    },
  })
  const review = useMutation({
    mutationFn: () =>
      isTauriRuntime()
        ? invokeValidated(
            "generate_weekly_review",
            { input: { provider: "codex" } },
            z.object({ provider: z.enum(["codex", "claude"]) }),
            weeklyLearningSchema,
          )
        : Promise.resolve(
            weeklyLearningSchema.parse({
              observation: "There is not enough verified data yet.",
              learning: "Run one focused experiment before changing the ICP.",
              counterEvidence: [],
              confidence: 0.2,
              nextExperiment:
                "Publish one clear problem framing and measure qualified replies for seven days.",
            }),
          ),
    onSuccess: setLearning,
  })
  const accept = useMutation({
    mutationFn: () => {
      if (!learning) throw new Error("Generate a learning first")
      return isTauriRuntime()
        ? invokeValidated("accept_learning", { input: learning }, weeklyLearningSchema, z.string().uuid())
        : Promise.resolve(crypto.randomUUID())
    },
    onSuccess: refreshLoop,
  })

  if (!growth.data || !loop.data)
    return (
      <div className="page-state">
        <span className="pulse-dot" />
        <h1>Opening the controlled growth loop…</h1>
      </div>
    )

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Growth · local action and evidence ledger</p>
          <h1>Turn activity into learning.</h1>
        </div>
        <Badge tone="good">
          <ShieldCheck size={13} /> Exact-action approval
        </Badge>
      </header>

      <section className="growth-loop-summary" aria-label="Controlled growth loop summary">
        <SummaryCard
          icon={ClipboardCheck}
          label="Proposed"
          value={loop.data.totals.proposed}
          detail="Waiting for exact review"
        />
        <SummaryCard
          icon={CheckCircle2}
          label="Approved"
          value={loop.data.totals.approved}
          detail="Ready for founder action"
        />
        <SummaryCard
          icon={Activity}
          label="Completed"
          value={loop.data.totals.completed}
          detail="Waiting for measurement"
        />
        <SummaryCard
          icon={Gauge}
          label="Measured"
          value={loop.data.totals.measured}
          detail="Evidence in local memory"
        />
      </section>

      <section className="controlled-loop-layout">
        <div className="controlled-loop-column">
          <section className="panel icp-version-card">
            <div className="panel-heading">
              <span className="panel-icon">
                <ShieldCheck size={17} />
              </span>
              <div>
                <h2>Active ICP memory</h2>
                <p>Every action can point to the exact audience hypothesis being tested.</p>
              </div>
            </div>
            {loop.data.activeIcp ? (
              <>
                <Badge tone="good">
                  Version {loop.data.activeIcp.version} · {Math.round(loop.data.activeIcp.confidence * 100)}%
                  confidence
                </Badge>
                <h3>{loop.data.activeIcp.role}</h3>
                <p>{loop.data.activeIcp.urgentProblem}</p>
                <small>{loop.data.activeIcp.situation}</small>
              </>
            ) : (
              <p className="muted-copy">
                Accept an ICP hypothesis in Memory before relying on audience-fit conclusions.
              </p>
            )}
          </section>

          <form
            className="panel growth-action-form"
            onSubmit={(event) => {
              event.preventDefault()
              propose.mutate()
            }}
          >
            <div className="panel-heading">
              <span className="panel-icon">
                <Plus size={17} />
              </span>
              <div>
                <h2>Propose an action</h2>
                <p>Capture the action, reason, experiment, and success signal before doing it.</p>
              </div>
            </div>
            <div className="growth-form-row">
              <Field label="Action">
                <select
                  className="input"
                  value={draft.kind}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      kind: event.target.value as typeof initialDraft.kind,
                    }))
                  }
                >
                  <option value="research">Research</option>
                  <option value="follow">Follow</option>
                  <option value="comment">Comment</option>
                  <option value="post">Post</option>
                </select>
              </Field>
              <Field label="Platform">
                <select
                  className="input"
                  value={draft.platform}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      platform: event.target.value as typeof initialDraft.platform,
                    }))
                  }
                >
                  <option value="x">X</option>
                  <option value="linkedin">LinkedIn</option>
                  <option value="reddit">Reddit</option>
                </select>
              </Field>
            </div>
            <Field label="Action title">
              <Input
                value={draft.title}
                onChange={(event) => setDraft((current) => ({ ...current, title: event.target.value }))}
                placeholder="Join one relevant founder conversation"
                required
              />
            </Field>
            <Field label="Why this belongs in today’s queue">
              <Textarea
                rows={3}
                value={draft.rationale}
                onChange={(event) => setDraft((current) => ({ ...current, rationale: event.target.value }))}
                placeholder="The author and problem match ICP v2."
                required
              />
            </Field>
            <Field label="Target URL (optional)">
              <Input
                type="url"
                value={draft.targetUrl}
                onChange={(event) => setDraft((current) => ({ ...current, targetUrl: event.target.value }))}
                placeholder="https://x.com/founder/status/..."
              />
            </Field>
            <Field label="Exact action or content">
              <Textarea
                rows={5}
                value={draft.exactPayload}
                onChange={(event) =>
                  setDraft((current) => ({ ...current, exactPayload: event.target.value }))
                }
                placeholder="The exact comment, post, follow target, or research instruction."
                required
              />
            </Field>
            <Field label="Experiment hypothesis">
              <Textarea
                rows={2}
                value={draft.hypothesis}
                onChange={(event) => setDraft((current) => ({ ...current, hypothesis: event.target.value }))}
                placeholder="Specific operator insights will create qualified replies."
                required
              />
            </Field>
            <div className="growth-form-row metric-definition-row">
              <Field label="Success signal">
                <Input
                  value={draft.successMetric}
                  onChange={(event) =>
                    setDraft((current) => ({ ...current, successMetric: event.target.value }))
                  }
                  required
                />
              </Field>
              <Field label="Window (days)">
                <Input
                  type="number"
                  min={1}
                  max={365}
                  value={draft.evaluationWindowDays}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      evaluationWindowDays: Number(event.target.value),
                    }))
                  }
                  required
                />
              </Field>
            </div>
            {propose.isError && <InlineError message={propose.error.message} />}
            <Button type="submit" disabled={propose.isPending}>
              {propose.isPending ? "Saving locally…" : "Add to controlled queue"}
            </Button>
          </form>
        </div>

        <section className="growth-queue-column" aria-label="Daily Growth Queue">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Daily Growth Queue</p>
              <h2>Nothing acts silently.</h2>
            </div>
            <Badge>{loop.data.actions.length} tracked</Badge>
          </div>
          {loop.data.actions.length ? (
            <div className="growth-action-list">
              {loop.data.actions.map((action) => (
                <GrowthActionCard key={action.id} action={action} onChanged={refreshLoop} />
              ))}
            </div>
          ) : (
            <div className="growth-empty-queue">
              <FlaskConical size={24} />
              <h3>Start with one testable action.</h3>
              <p>
                Goalbar will preserve its exact revision, approval, completion evidence, metrics, and
                learning.
              </p>
            </div>
          )}
        </section>
      </section>

      <section className="growth-hero">
        <div className="big-score">
          <strong>{Math.round(growth.data.score)}</strong>
          <span>Sustainable Growth Score</span>
          <small>Formula version {growth.data.formulaVersion} · missing data is not treated as zero</small>
        </div>
        <div className="component-list">
          {componentLabels.map((key) => {
            const value = growth.data.components[key]
            return (
              <div className="component-row" key={key}>
                <span>{titleCase(key)}</span>
                <div className="metric-track">
                  <i style={{ width: `${value ?? 0}%` }} />
                </div>
                <strong>{value == null ? "Missing" : Math.round(value)}</strong>
              </div>
            )
          })}
        </div>
      </section>

      <section className="panel">
        <p className="eyebrow">What Goalbar learned</p>
        <h2>Observation stays separate from interpretation.</h2>
        {loop.data.learnings.length > 0 && (
          <div className="tracked-learning-list">
            {loop.data.learnings.slice(0, 5).map((item) => (
              <article className="learning-card" key={item.id}>
                <Badge tone={item.confidence >= 0.7 ? "good" : "warn"}>
                  {Math.round(item.confidence * 100)}% confidence
                </Badge>
                <h3>{item.summary}</h3>
                <p>{learningObservation(item.evidence)}</p>
              </article>
            ))}
          </div>
        )}
        {learning && (
          <article className="learning-card">
            <Badge tone="warn">{Math.round(learning.confidence * 100)}% confidence</Badge>
            <h3>{learning.learning}</h3>
            <p>{learning.observation}</p>
            <strong>Next experiment</strong>
            <p>{learning.nextExperiment}</p>
            <Button
              size="small"
              onClick={() => accept.mutate()}
              disabled={accept.isPending || accept.isSuccess}
            >
              {accept.isSuccess ? "Learning accepted" : "Accept into local memory"}
            </Button>
          </article>
        )}
        <Button
          className="memory-button"
          size="small"
          variant="secondary"
          onClick={() => review.mutate()}
          disabled={review.isPending}
        >
          {review.isPending ? "Interpreting verified inputs…" : "Generate weekly review"}
        </Button>
      </section>
    </div>
  )
}

function GrowthActionCard({ action, onChanged }: { action: GrowthAction; onChanged: () => Promise<void> }) {
  const [payload, setPayload] = useState(action.exactPayload)
  const [resultUrl, setResultUrl] = useState("")
  const [metricName, setMetricName] = useState("qualified_replies")
  const [metricValue, setMetricValue] = useState("")
  const [observation, setObservation] = useState("")
  const [learning, setLearning] = useState("")
  const [nextExperiment, setNextExperiment] = useState("")
  const revised = payload !== action.exactPayload
  const revise = useMutation({
    mutationFn: () => {
      const input = { actionId: action.id, exactPayload: payload }
      return isTauriRuntime()
        ? invokeValidated(
            "revise_growth_action",
            { input },
            reviseGrowthActionInputSchema,
            growthActionSchema,
          )
        : Promise.resolve({ ...action, exactPayload: payload, revision: action.revision + 1 })
    },
    onSuccess: onChanged,
  })
  const approve = useMutation({
    mutationFn: () => {
      const input = { actionId: action.id, exactPayload: action.exactPayload }
      return isTauriRuntime()
        ? invokeValidated("approve_growth_action", { input }, approveGrowthActionInputSchema, approvalSchema)
        : Promise.resolve(
            approvalSchema.parse({
              id: crypto.randomUUID(),
              subjectType: "growth_action",
              subjectId: action.id,
              payloadHash: action.payloadHash,
              idempotencyKey: crypto.randomUUID(),
              approvedAt: new Date().toISOString(),
            }),
          )
    },
    onSuccess: onChanged,
  })
  const complete = useMutation({
    mutationFn: () => {
      if (!action.approvalId) throw new Error("Approve this exact revision first")
      const input = {
        actionId: action.id,
        approvalId: action.approvalId,
        exactPayload: action.exactPayload,
        outcome: "succeeded" as const,
        resultUrl: resultUrl || undefined,
        detail: "Founder confirmed the approved action was completed.",
      }
      return isTauriRuntime()
        ? invokeValidated(
            "record_growth_action_execution",
            { input },
            recordGrowthActionExecutionInputSchema,
            growthActionSchema,
          )
        : Promise.resolve({ ...action, status: "completed" as const })
    },
    onSuccess: onChanged,
  })
  const measure = useMutation({
    mutationFn: () => {
      const input = {
        actionId: action.id,
        metricName,
        value: Number(metricValue),
        availability: "available" as const,
        sourceDefinition: "Founder-entered observation from the platform UI.",
        notes: "",
        observedAt: new Date().toISOString(),
      }
      return isTauriRuntime()
        ? invokeValidated(
            "record_growth_action_metric",
            { input },
            recordGrowthActionMetricInputSchema,
            growthActionSchema,
          )
        : Promise.resolve({ ...action, status: "measured" as const })
    },
    onSuccess: async () => {
      setMetricValue("")
      await onChanged()
    },
  })
  const learn = useMutation({
    mutationFn: () => {
      const input = {
        actionId: action.id,
        observation,
        learning,
        counterEvidence: [],
        confidence: 0.5,
        nextExperiment,
      }
      return isTauriRuntime()
        ? invokeValidated(
            "record_growth_action_learning",
            { input },
            recordGrowthLearningInputSchema,
            trackedGrowthLearningSchema,
          )
        : Promise.resolve(
            trackedGrowthLearningSchema.parse({
              id: crypto.randomUUID(),
              growthActionId: action.id,
              summary: learning,
              evidence: { observation, counterEvidence: [], nextExperiment },
              confidence: 0.5,
              status: "accepted",
              createdAt: new Date().toISOString(),
            }),
          )
    },
    onSuccess: async () => {
      setObservation("")
      setLearning("")
      setNextExperiment("")
      await onChanged()
    },
  })
  const error = revise.error ?? approve.error ?? complete.error ?? measure.error ?? learn.error

  return (
    <article className="growth-action-card">
      <div className="growth-action-card-header">
        <div>
          <div className="growth-action-badges">
            <Badge tone={statusTone(action.status)}>{titleCase(action.status)}</Badge>
            <Badge>{action.platform ? titleCase(action.platform) : "Local"}</Badge>
            <span>Revision {action.revision}</span>
          </div>
          <h3>{action.title}</h3>
          <p>{action.rationale}</p>
        </div>
        <strong>{titleCase(action.kind)}</strong>
      </div>
      <dl className="growth-action-definition">
        <div>
          <dt>Hypothesis</dt>
          <dd>{action.hypothesis}</dd>
        </div>
        <div>
          <dt>Success</dt>
          <dd>
            {action.successMetric} · {action.evaluationWindowDays} days
          </dd>
        </div>
      </dl>
      {action.targetUrl && (
        <span className="growth-target-link" title={action.targetUrl}>
          Target: {action.targetUrl}
        </span>
      )}
      <label className="field growth-exact-payload">
        <span>Exact approved payload</span>
        <Textarea
          rows={4}
          value={payload}
          onChange={(event) => setPayload(event.target.value)}
          disabled={!matchesEditable(action.status)}
        />
      </label>
      <div className="growth-action-controls">
        {revised && matchesEditable(action.status) ? (
          <Button size="small" onClick={() => revise.mutate()} disabled={revise.isPending}>
            Save as revision {action.revision + 1}
          </Button>
        ) : action.status === "proposed" ? (
          <Button size="small" onClick={() => approve.mutate()} disabled={approve.isPending}>
            Approve exact revision
          </Button>
        ) : null}
        {action.status === "approved" && (
          <>
            <Input
              type="url"
              value={resultUrl}
              onChange={(event) => setResultUrl(event.target.value)}
              placeholder="Result URL (optional)"
            />
            <Button size="small" onClick={() => complete.mutate()} disabled={complete.isPending}>
              Record as completed
            </Button>
          </>
        )}
      </div>
      {(action.status === "completed" || action.status === "measured") && (
        <div className="growth-measure-form">
          <label>
            <span>Metric</span>
            <select
              className="input"
              value={metricName}
              onChange={(event) => setMetricName(event.target.value)}
            >
              <option value="qualified_replies">Qualified replies</option>
              <option value="attention_quality">Attention quality (0–100)</option>
              <option value="conversation_quality">Conversation quality (0–100)</option>
              <option value="relationship_growth">Relationship growth (0–100)</option>
              <option value="consistency">Consistency (0–100)</option>
              <option value="learning_velocity">Learning velocity (0–100)</option>
            </select>
          </label>
          <label>
            <span>Observed value</span>
            <Input
              type="number"
              min={0}
              step="any"
              value={metricValue}
              onChange={(event) => setMetricValue(event.target.value)}
            />
          </label>
          <Button
            size="small"
            variant="secondary"
            onClick={() => measure.mutate()}
            disabled={!metricValue || measure.isPending}
          >
            Record metric
          </Button>
        </div>
      )}
      {action.metrics.length > 0 && (
        <div className="growth-metric-history">
          {action.metrics.map((metric) => (
            <span key={metric.id}>
              {titleCase(metric.metricName)}:{" "}
              <strong>{metric.value == null ? titleCase(metric.availability) : metric.value}</strong>
            </span>
          ))}
        </div>
      )}
      {action.status === "measured" && (
        <details className="growth-learning-form">
          <summary>Record an evidence-backed learning</summary>
          <Field label="Observation">
            <Textarea rows={2} value={observation} onChange={(event) => setObservation(event.target.value)} />
          </Field>
          <Field label="Learning">
            <Textarea rows={2} value={learning} onChange={(event) => setLearning(event.target.value)} />
          </Field>
          <Field label="Next experiment">
            <Input value={nextExperiment} onChange={(event) => setNextExperiment(event.target.value)} />
          </Field>
          <Button
            size="small"
            onClick={() => learn.mutate()}
            disabled={!observation || !learning || !nextExperiment || learn.isPending}
          >
            Accept learning at 50% confidence
          </Button>
        </details>
      )}
      {error && <InlineError message={error.message} />}
    </article>
  )
}

function SummaryCard({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof ClipboardCheck
  label: string
  value: number
  detail: string
}) {
  return (
    <article>
      <span>
        <Icon size={16} />
      </span>
      <div>
        <small>{label}</small>
        <strong>{value}</strong>
        <p>{detail}</p>
      </div>
    </article>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
    </label>
  )
}

function InlineError({ message }: { message: string }) {
  return (
    <div className="inline-error">
      <strong>Local growth loop needs attention</strong>
      <span>{message}</span>
    </div>
  )
}

function statusTone(status: GrowthAction["status"]): "good" | "warn" | "neutral" | "danger" {
  if (status === "measured" || status === "completed") return "good"
  if (status === "failed" || status === "cancelled") return "danger"
  if (status === "approved") return "warn"
  return "neutral"
}

function matchesEditable(status: GrowthAction["status"]) {
  return status === "proposed" || status === "approved"
}

function learningObservation(evidence: unknown) {
  if (
    evidence &&
    typeof evidence === "object" &&
    "observation" in evidence &&
    typeof evidence.observation === "string"
  ) {
    return evidence.observation
  }
  return "Stored with its source evidence in local memory."
}

function previewGrowthLoop(): GrowthLoopOverview {
  return growthLoopOverviewSchema.parse({
    schemaVersion: 1,
    activeIcp: null,
    actions: [],
    learnings: [],
    totals: { proposed: 0, approved: 0, completed: 0, measured: 0 },
  })
}

function previewAction(input: ProposeGrowthActionInput): GrowthAction {
  return growthActionSchema.parse({
    id: crypto.randomUUID(),
    founderId: crypto.randomUUID(),
    icpHypothesisId: input.icpHypothesisId,
    experimentId: input.experimentId,
    kind: input.kind,
    platform: input.platform,
    title: input.title,
    rationale: input.rationale,
    targetUrl: input.targetUrl,
    exactPayload: input.exactPayload,
    payloadHash: "local-preview",
    revision: 1,
    hypothesis: input.hypothesis,
    successMetric: input.successMetric,
    evaluationWindowDays: input.evaluationWindowDays,
    status: "proposed",
    scheduledFor: input.scheduledFor,
    completedAt: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    approvalId: null,
    executions: [],
    metrics: [],
  })
}
