import { useMutation, useQuery } from "@tanstack/react-query"
import { useState } from "react"
import { z } from "zod"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { queryKeys } from "@/lib/query-keys"
import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { titleCase } from "@/lib/utils"
import { growthScoreSchema, weeklyLearningSchema } from "@/schemas/growth"

const componentLabels = [
  "attentionQuality",
  "conversationQuality",
  "relationshipGrowth",
  "consistency",
  "learningVelocity",
] as const

export function GrowthPage() {
  const [learning, setLearning] = useState<z.infer<typeof weeklyLearningSchema> | null>(null)
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
  })
  if (!growth.data)
    return (
      <div className="page-state">
        <span className="pulse-dot" />
        <h1>Calculating verified signals…</h1>
      </div>
    )
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Growth · 28-day evidence window</p>
          <h1>Progress you can explain.</h1>
        </div>
        <Badge tone={growth.data.confidence > 0.6 ? "good" : "warn"}>
          {Math.round(growth.data.confidence * 100)}% confidence
        </Badge>
      </header>
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
        <p className="eyebrow">Weekly learning</p>
        <h2>Evidence before interpretation.</h2>
        <p className="muted-copy">
          Rust calculates the window and score. Codex or Claude can propose a learning only from those
          verified inputs.
        </p>
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
