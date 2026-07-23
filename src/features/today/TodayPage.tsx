import { ArrowRight, Bot, Cable, RefreshCw, ShieldCheck } from "lucide-react"
import { Link } from "react-router-dom"

import { useBootstrap } from "@/app/bootstrap"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { titleCase } from "@/lib/utils"

export function TodayPage() {
  const bootstrap = useBootstrap()
  if (bootstrap.isPending) return <PageState title="Opening your local lab…" />
  if (bootstrap.isError)
    return (
      <PageState
        title="The local core is unavailable"
        detail={bootstrap.error.message}
        onRetry={() => void bootstrap.refetch()}
      />
    )

  const state = bootstrap.data
  const primary = state.nextActions[0]
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Today · local command center</p>
          <h1>
            {state.founder ? `Good to see you, ${state.founder.name}.` : "Build a growth loop you can trust."}
          </h1>
        </div>
        <Button variant="secondary" onClick={() => void bootstrap.refetch()}>
          <RefreshCw size={15} /> Sync view
        </Button>
      </header>

      <section className="hero-grid">
        <article className="next-action-card">
          <div className="card-topline">
            <span>Recommended next action</span>
            <Badge tone="good">Why included</Badge>
          </div>
          <h2>{primary?.title ?? "Your queue is clear"}</h2>
          <p>{primary?.reason ?? "Create an experiment when you have a fresh founder insight."}</p>
          {primary && (
            <Link className="text-link" to={primary.route}>
              Continue <ArrowRight size={15} />
            </Link>
          )}
        </article>
        <article className="score-card">
          <div className="score-ring" style={{ "--score": `${state.score.score}%` } as React.CSSProperties}>
            <strong>{Math.round(state.score.score)}</strong>
            <span>/ 100</span>
          </div>
          <div>
            <p className="eyebrow">Sustainable Growth Score</p>
            <h3>{state.score.confidence ? "Evidence is accumulating" : "Waiting for real signals"}</h3>
            <p>
              {Math.round(state.score.confidence * 100)}% metric confidence · formula v
              {state.score.formulaVersion}
            </p>
          </div>
        </article>
      </section>

      <section>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Control plane</p>
            <h2>Your system at a glance</h2>
          </div>
        </div>
        <div className="status-grid">
          <StatusCard
            icon={Bot}
            title="Reasoning engines"
            value={`${state.agents.filter((agent) => agent.readiness === "ready").length} ready`}
            detail={state.agents
              .map((agent) => `${titleCase(agent.provider)}: ${titleCase(agent.readiness)}`)
              .join(" · ")}
          />
          <StatusCard
            icon={Cable}
            title="Founder channels"
            value={`${state.accounts.length} connected`}
            detail={
              state.accounts.length
                ? state.accounts.map((account) => titleCase(account.platform)).join(" · ")
                : "Connect through local OAuth when ready"
            }
          />
          <StatusCard
            icon={ShieldCheck}
            title="Credential custody"
            value="On this machine"
            detail="Tokens stay in the OS keyring; agents never receive them"
          />
        </div>
      </section>

      {state.nextActions.length > 1 && (
        <section>
          <div className="section-heading">
            <div>
              <p className="eyebrow">Queue</p>
              <h2>After that</h2>
            </div>
          </div>
          <div className="action-list">
            {state.nextActions.slice(1).map((action) => (
              <Link to={action.route} key={`${action.kind}-${action.title}`} className="action-row">
                <span>
                  <strong>{action.title}</strong>
                  <small>{action.reason}</small>
                </span>
                <ArrowRight size={16} />
              </Link>
            ))}
          </div>
        </section>
      )}
    </div>
  )
}

function StatusCard({
  icon: Icon,
  title,
  value,
  detail,
}: {
  icon: typeof Bot
  title: string
  value: string
  detail: string
}) {
  return (
    <article className="status-card">
      <span className="status-icon">
        <Icon size={18} />
      </span>
      <p>{title}</p>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  )
}

function PageState({ title, detail, onRetry }: { title: string; detail?: string; onRetry?: () => void }) {
  return (
    <div className="page-state">
      <span className="pulse-dot" />
      <h1>{title}</h1>
      {detail && <p>{detail}</p>}
      {onRetry && <Button onClick={onRetry}>Try again</Button>}
    </div>
  )
}
