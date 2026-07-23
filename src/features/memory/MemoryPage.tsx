import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { BrainCircuit, Check, Sparkles } from "lucide-react"
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
  acceptIcpInputSchema,
  icpHypothesesSchema,
  saveVoiceInputSchema,
  storedIcpHypothesesSchema,
} from "@/schemas/memory"
import { historyOverviewSchema } from "@/schemas/history"

export function MemoryPage() {
  const { data } = useBootstrap()
  const queryClient = useQueryClient()
  const [example, setExample] = useState("")
  const [traits, setTraits] = useState("direct, curious, practical")
  const [icp, setIcp] = useState<z.infer<typeof icpHypothesesSchema> | null>(null)
  const provider = data?.agents.find((agent) => agent.readiness === "ready")?.provider ?? "codex"
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
  const discoverIcp = useMutation({
    mutationFn: async () => {
      const input = { provider }
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
        z.object({ provider: z.enum(["codex", "claude"]) }),
        icpHypothesesSchema,
      )
    },
    onSuccess: async (value) => {
      setIcp(value)
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
          <p className="eyebrow">Founder baseline</p>
          <h2>{data?.founder?.productName ?? "Not configured"}</h2>
          <p className="muted-copy">
            {data?.founder?.offer ?? "Complete onboarding to establish an offer and voice boundary."}
          </p>
          {data?.founder && (
            <dl className="definition-list">
              <div>
                <dt>Founder</dt>
                <dd>{data.founder.name}</dd>
              </div>
              <div>
                <dt>Expertise</dt>
                <dd>{data.founder.expertise}</dd>
              </div>
              <div>
                <dt>Goal</dt>
                <dd>{data.founder.goals[0] ?? "Not set"}</dd>
              </div>
            </dl>
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
          <div className="panel-heading">
            <span className="panel-icon">
              <Sparkles size={17} />
            </span>
            <div>
              <h2>ICP hypotheses</h2>
              <p>Evidence-seeking guesses, never silent truth.</p>
            </div>
          </div>
          {displayedIcp.length > 0 ? (
            <div className="hypothesis-list">
              {displayedIcp.map((hypothesis) => (
                <article key={`${hypothesis.role}-${hypothesis.situation}`}>
                  <Badge tone={hypothesis.status === "active" ? "good" : "warn"}>
                    v{hypothesis.version} · {hypothesis.status === "active" ? "Active · " : "Proposed · "}
                    {Math.round(hypothesis.confidence * 100)}% confidence
                  </Badge>
                  <h3>{hypothesis.role}</h3>
                  <p>{hypothesis.urgentProblem}</p>
                  <small>{hypothesis.situation}</small>
                  {hypothesis.id && hypothesis.status === "proposed" && (
                    <Button
                      size="small"
                      variant="secondary"
                      onClick={() => acceptIcp.mutate(hypothesis.id)}
                      disabled={acceptIcp.isPending}
                    >
                      Accept as active ICP
                    </Button>
                  )}
                </article>
              ))}
            </div>
          ) : (
            <p className="muted-copy">
              Ask {provider} to propose an initial segment, urgent problem, current workaround, desired
              outcome, language, and objections from your approved baseline.
            </p>
          )}
          <Button
            className="memory-button"
            size="small"
            variant="secondary"
            onClick={() => discoverIcp.mutate()}
            disabled={!data?.founder || discoverIcp.isPending}
          >
            {discoverIcp.isPending ? "Discovering…" : `Discover with ${provider}`}
          </Button>
        </section>
      </div>
    </div>
  )
}
