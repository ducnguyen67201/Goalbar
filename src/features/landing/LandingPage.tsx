import {
  ArrowRight,
  Check,
  ChevronDown,
  CircleCheck,
  Command,
  Download,
  Github,
  LockKeyhole,
  MousePointer2,
  Play,
  Search,
  Sparkles,
  Star,
} from "lucide-react"
import { type FormEvent, useEffect, useState } from "react"
import { Link, useLocation } from "react-router-dom"

import {
  captureMarketingEvent,
  captureMarketingPageView,
  type MarketingAnalyticsEvent,
} from "@/lib/marketing-analytics"

const goalbarRepositoryUrl = "https://github.com/ducnguyen67201/Goalbar"
const goalbarRepositoryApiUrl = "https://api.github.com/repos/ducnguyen67201/Goalbar"
const initialGitHubStars = 0

const platforms = [
  { name: "LinkedIn", mark: "in", className: "is-linkedin" },
  { name: "X", mark: "𝕏", className: "is-x" },
  { name: "Reddit", mark: "r/", className: "is-reddit" },
]

const capabilities = [
  {
    tag: "FIND THE ROOM",
    title: "Find the signal.",
    copy: "Spot conversations worth joining.",
    accent: "lime",
  },
  {
    tag: "SOUND LIKE YOU",
    title: "Write like you.",
    copy: "Your voice and context. Never a canned template.",
    accent: "orange",
  },
  {
    tag: "CLOSE THE LOOP",
    title: "Get sharper.",
    copy: "Every reply improves the next.",
    accent: "blue",
  },
]

export function LandingPage() {
  const [sceneKey, setSceneKey] = useState(0)
  const githubStars = useGitHubStars()
  const location = useLocation()

  useEffect(() => {
    void captureMarketingPageView()
  }, [location.pathname])

  return (
    <div className="gb-landing">
      <header className="gb-nav">
        <a className="gb-wordmark" href="#top" aria-label="Goalbar home">
          <span className="gb-logo-mark">
            <img src="/goalbar-logo-8bit.png" alt="" aria-hidden="true" />
          </span>
          <span>goalbar</span>
        </a>

        <nav aria-label="Landing page navigation">
          <a href="#why-goalbar">Why Goalbar</a>
        </nav>

        <div className="gb-nav-actions">
          <a
            className="gb-nav-github"
            href={goalbarRepositoryUrl}
            target="_blank"
            rel="noreferrer"
            aria-label={`Star Goalbar on GitHub, ${githubStars} ${githubStars === 1 ? "star" : "stars"}`}
            onClick={() => trackCta("github", "nav")}
          >
            <Github size={15} aria-hidden="true" />
            <span className="gb-github-label">GitHub</span>
            <span className="gb-github-stars" aria-live="polite">
              <Star size={12} fill="currentColor" aria-hidden="true" />
              {formatStarCount(githubStars)}
            </span>
          </a>
          <a className="gb-nav-download" href="#download" onClick={() => trackCta("download", "nav")}>
            Get download <Download size={14} aria-hidden="true" />
          </a>
          <Link className="gb-nav-cta" to="/" onClick={() => trackCta("open_app", "nav")}>
            Open the app <ArrowRight size={15} aria-hidden="true" />
          </Link>
        </div>
      </header>

      <main>
        <section className="gb-hero" id="top">
          <SideComputer />
          <div className="gb-hero-copy">
            <p className="gb-kicker">
              <span />
              Content + outbound, on your machine
            </p>
            <h1>
              Your GTM
              <br />
              <span className="gb-drag-word">
                <em>cofounder.</em>
                <span className="gb-drop-cursor" aria-hidden="true">
                  <MousePointer2 />
                  <small>DROP</small>
                </span>
              </span>
            </h1>
            <p className="gb-hero-deck">
              Finds the right conversations. Drafts the reply. You approve the send.
            </p>
            <DownloadForm placement="hero" />
            <div className="gb-hero-actions">
              <a
                className="gb-button gb-button-ghost"
                href="#why-goalbar"
                onClick={() => trackCta("why_goalbar", "hero")}
              >
                Why Goalbar <ChevronDown size={17} aria-hidden="true" />
              </a>
              <Link className="gb-hero-open-link" to="/" onClick={() => trackCta("open_app", "hero")}>
                Already installed? Open app <ArrowRight size={15} aria-hidden="true" />
              </Link>
            </div>
            <div className="gb-trust-line">
              <span>
                <LockKeyhole size={13} aria-hidden="true" /> Your sessions stay local
              </span>
              <span>
                <CircleCheck size={13} aria-hidden="true" /> You approve every send
              </span>
            </div>
          </div>

          <div
            className="gb-demo-wrap"
            aria-label="Animated preview of Goalbar working across social platforms"
          >
            <div className="gb-orbit-label gb-orbit-label-one">reads the room</div>
            <div className="gb-orbit-label gb-orbit-label-two">writes like you</div>
            <div className="gb-demo-window" key={sceneKey} aria-hidden="true">
              <div className="gb-window-chrome">
                <span className="gb-window-dots" aria-hidden="true">
                  <i />
                  <i />
                  <i />
                </span>
                <span className="gb-window-title">
                  <Sparkles size={12} aria-hidden="true" /> Goalbar mission
                </span>
                <span className="gb-live-pill">
                  <i /> working
                </span>
              </div>

              <div className="gb-window-body">
                <aside className="gb-platform-rail" aria-label="Connected platforms">
                  <span className="gb-rail-logo">
                    <img src="/goalbar-logo-8bit.png" alt="" />
                  </span>
                  {platforms.map((platform) => (
                    <span
                      className={`gb-platform-mark ${platform.className}`}
                      key={platform.name}
                      title={platform.name}
                    >
                      {platform.mark}
                    </span>
                  ))}
                  <span className="gb-rail-avatar">DN</span>
                </aside>

                <div className="gb-workspace">
                  <div className="gb-mission-bar">
                    <span className="gb-mission-icon">
                      <Command size={14} aria-hidden="true" />
                    </span>
                    <span>
                      <small>Today’s mission</small>
                      <strong>Join 3 founder conversations with something useful</strong>
                    </span>
                    <span className="gb-progress">2 / 3</span>
                  </div>

                  <div className="gb-browser-grid">
                    <article className="gb-feed-panel">
                      <div className="gb-panel-head">
                        <span className="gb-platform-mark is-linkedin">in</span>
                        <span>Founder feed</span>
                        <span className="gb-panel-more">•••</span>
                      </div>
                      <div className="gb-post-author">
                        <span className="gb-avatar gb-avatar-blue">MA</span>
                        <span>
                          <strong>Maya A.</strong>
                          <small>building in public · 12m</small>
                        </span>
                      </div>
                      <p className="gb-post-copy">
                        We changed one sentence in our onboarding and activation jumped. Positioning really is
                        product work.
                      </p>
                      <div className="gb-post-metrics">
                        <span>◎ 48</span>
                        <span>12 comments</span>
                      </div>
                      <button className="gb-comment-trigger" type="button" tabIndex={-1}>
                        <MousePointer2 size={13} aria-hidden="true" /> Comment
                      </button>
                    </article>

                    <article className="gb-compose-panel">
                      <div className="gb-panel-head">
                        <span>Reply draft</span>
                        <span className="gb-context-pill">3 memories used</span>
                      </div>
                      <div className="gb-draft-box">
                        <span className="gb-avatar gb-avatar-lime">YO</span>
                        <p>
                          The best positioning changes feel almost too obvious after the fact. Curious—did the
                          new line change who activated, or just how many?
                          <span className="gb-type-caret" />
                        </p>
                      </div>
                      <div className="gb-tone-row">
                        <span>your voice</span>
                        <span>specific</span>
                        <span>no pitch</span>
                      </div>
                      <div className="gb-approval-row">
                        <button type="button" tabIndex={-1}>
                          Edit
                        </button>
                        <button className="gb-approve-button" type="button" tabIndex={-1}>
                          <Check size={14} aria-hidden="true" /> Approve reply
                        </button>
                      </div>
                    </article>
                  </div>

                  <div className="gb-status-strip">
                    <span className="gb-status-platform">
                      <span className="gb-platform-mark is-x">𝕏</span>
                      Found a relevant thread
                    </span>
                    <span className="gb-status-platform">
                      <span className="gb-platform-mark is-reddit">r/</span>
                      Reading 18 comments
                    </span>
                    <span className="gb-status-platform is-done">
                      <CircleCheck size={13} aria-hidden="true" />
                      LinkedIn draft ready
                    </span>
                  </div>
                </div>

                <span className="gb-agent-cursor gb-cursor-scout" aria-hidden="true">
                  <MousePointer2 />
                  <small>scout</small>
                  <i />
                </span>
                <span className="gb-agent-cursor gb-cursor-writer" aria-hidden="true">
                  <MousePointer2 />
                  <small>writer</small>
                  <i />
                </span>
                <span className="gb-agent-cursor gb-cursor-you" aria-hidden="true">
                  <MousePointer2 />
                  <small>you</small>
                  <i />
                </span>
              </div>
            </div>
            <button
              className="gb-replay-button"
              type="button"
              onClick={() => {
                setSceneKey((current) => current + 1)
                void captureMarketingEvent({
                  name: "marketing_demo_replayed",
                  properties: { placement: "hero" },
                })
              }}
            >
              <Play size={13} fill="currentColor" aria-hidden="true" /> Replay the clicks
            </button>
          </div>
        </section>

        <section className="gb-capabilities" id="why-goalbar">
          <div className="gb-capability-intro">
            <p className="gb-kicker">
              <span />
              Why Goalbar
            </p>
            <h2>Less content grind. More real conversations.</h2>
          </div>

          <div className="gb-capability-grid">
            {capabilities.map((capability, index) => (
              <article className={`gb-capability-card is-${capability.accent}`} key={capability.tag}>
                <div className="gb-capability-top">
                  <span>{capability.tag}</span>
                  <span>0{index + 1}</span>
                </div>
                <div className="gb-capability-visual" aria-hidden="true">
                  {index === 0 && (
                    <>
                      <span className="gb-search-pill">
                        <Search size={13} /> founder-led growth
                      </span>
                      <span className="gb-signal-line is-one" />
                      <span className="gb-signal-line is-two" />
                      <span className="gb-signal-line is-three" />
                      <span className="gb-mini-cursor">
                        <MousePointer2 size={20} />
                      </span>
                    </>
                  )}
                  {index === 1 && (
                    <div className="gb-voice-note">
                      <span>Not “Great post!”</span>
                      <strong>Ask the question only you would ask.</strong>
                      <i />
                    </div>
                  )}
                  {index === 2 && (
                    <div className="gb-loop-visual">
                      <span>signal</span>
                      <i />
                      <span>reply</span>
                      <i />
                      <span>learn</span>
                      <b>↻</b>
                    </div>
                  )}
                </div>
                <h3>{capability.title}</h3>
                <p>{capability.copy}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="gb-final-cta" id="download">
          <div className="gb-cta-cursor gb-cta-cursor-one" aria-hidden="true">
            <MousePointer2 />
            <span>found it</span>
          </div>
          <div className="gb-cta-cursor gb-cta-cursor-two" aria-hidden="true">
            <MousePointer2 />
            <span>drafted it</span>
          </div>
          <p>Ready when you are</p>
          <h2>
            Start more
            <br />
            <em>conversations.</em>
          </h2>
          <DownloadForm placement="final" />
        </section>
      </main>

      <footer className="gb-footer">
        <a className="gb-wordmark" href="#top">
          <span className="gb-logo-mark">
            <img src="/goalbar-logo-8bit.png" alt="" aria-hidden="true" />
          </span>
          <span>goalbar</span>
        </a>
        <p>Your local GTM cofounder.</p>
        <div>
          <a href="#why-goalbar" onClick={() => trackCta("why_goalbar", "footer")}>
            Why Goalbar
          </a>
          <Link to="/" onClick={() => trackCta("open_app", "footer")}>
            Open app
          </Link>
        </div>
      </footer>
    </div>
  )
}

function useGitHubStars() {
  const [stars, setStars] = useState(initialGitHubStars)

  useEffect(() => {
    if (import.meta.env.MODE === "test") return

    const controller = new AbortController()

    async function loadStars() {
      try {
        const response = await fetch(goalbarRepositoryApiUrl, {
          headers: { Accept: "application/vnd.github+json" },
          signal: controller.signal,
        })
        if (!response.ok) return

        const repository = (await response.json()) as { stargazers_count?: unknown }
        if (typeof repository.stargazers_count === "number") {
          setStars(repository.stargazers_count)
        }
      } catch (error) {
        if (!(error instanceof DOMException && error.name === "AbortError")) {
          return
        }
      }
    }

    void loadStars()
    return () => controller.abort()
  }, [])

  return stars
}

function formatStarCount(stars: number) {
  if (stars < 1_000) return stars.toString()
  if (stars < 10_000) return `${(stars / 1_000).toFixed(1)}k`
  return `${Math.round(stars / 1_000)}k`
}

function SideComputer() {
  return (
    <div className="gb-side-computer" aria-hidden="true">
      <div className="gb-side-computer-head">
        <span>
          <i />
          <i />
          <i />
        </span>
        SIGNAL SCOUT.EXE
      </div>
      <div className="gb-side-computer-screen">
        <div className="gb-side-platform">𝕏</div>
        <div className="gb-side-post">
          <span>LIVE THREAD · 28 REPLIES</span>
          <strong>Founders are debating distribution again.</strong>
          <i />
          <i />
          <i />
        </div>
        <div className="gb-side-match">
          <span>92%</span>
          VOICE MATCH
        </div>
        <span className="gb-side-cursor">
          <MousePointer2 />
          <small>SCOUT</small>
        </span>
      </div>
      <div className="gb-side-computer-status">
        <i />
        READING CONTEXT...
      </div>
      <div className="gb-side-computer-base">
        <span />
      </div>
    </div>
  )
}

function DownloadForm({ placement }: { placement: "hero" | "final" }) {
  const [email, setEmail] = useState("")
  const [submitted, setSubmitted] = useState(false)

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setSubmitted(true)
    void captureMarketingEvent({
      name: "marketing_download_requested",
      properties: { placement },
    })
  }

  if (submitted) {
    return (
      <div className={`gb-download-success is-${placement}`} role="status" aria-live="polite">
        <span>
          <CircleCheck size={18} aria-hidden="true" />
        </span>
        <div>
          <strong>You’re on the download list.</strong>
          <small>Held in this local preview.</small>
        </div>
        <button type="button" onClick={() => setSubmitted(false)}>
          Change email
        </button>
      </div>
    )
  }

  return (
    <form
      className={`gb-download-form is-${placement}`}
      aria-label={`${placement === "hero" ? "Hero" : "Final"} download signup`}
      onSubmit={handleSubmit}
    >
      <label htmlFor={`download-email-${placement}`}>Email address</label>
      <div className="gb-download-control">
        <input
          id={`download-email-${placement}`}
          name="email"
          type="email"
          autoComplete="email"
          placeholder="you@company.com"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          required
        />
        <button type="submit">
          Email me the download <Download size={15} aria-hidden="true" />
        </button>
      </div>
      {placement === "hero" && (
        <span className="gb-download-nudge" aria-hidden="true">
          <MousePointer2 />
          <small>Hey! Click to download</small>
        </span>
      )}
      <small>
        <LockKeyhole size={11} aria-hidden="true" />
        Mac preview · local form only
      </small>
    </form>
  )
}

function trackCta(
  cta: Extract<MarketingAnalyticsEvent, { name: "marketing_cta_clicked" }>["properties"]["cta"],
  placement: Extract<MarketingAnalyticsEvent, { name: "marketing_cta_clicked" }>["properties"]["placement"],
) {
  void captureMarketingEvent({
    name: "marketing_cta_clicked",
    properties: { cta, placement },
  })
}
