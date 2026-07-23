import type { PostHog } from "posthog-js"

import { isTauriRuntime } from "@/lib/tauri"

type MarketingPlacement = "nav" | "hero" | "final" | "footer"

export type MarketingAnalyticsEvent =
  | {
      name: "marketing_cta_clicked"
      properties: {
        cta: "download" | "github" | "open_app" | "why_goalbar"
        placement: MarketingPlacement
      }
    }
  | {
      name: "marketing_demo_replayed"
      properties: { placement: "hero" }
    }
  | {
      name: "marketing_download_requested"
      properties: { placement: "hero" | "final" }
    }

let clientPromise: Promise<PostHog | null> | null = null

function loadClient() {
  if (clientPromise) return clientPromise

  const projectKey = import.meta.env.VITE_POSTHOG_KEY?.trim()
  if (!projectKey || typeof window === "undefined" || isTauriRuntime()) {
    return Promise.resolve(null)
  }

  const apiHost = import.meta.env.VITE_POSTHOG_HOST?.trim() || "https://us.i.posthog.com"

  clientPromise = import("posthog-js")
    .then(({ default: posthog }) => {
      posthog.init(projectKey, {
        api_host: apiHost,
        person_profiles: "identified_only",
        persistence: "memory",
        autocapture: false,
        rageclick: false,
        capture_pageview: false,
        capture_pageleave: true,
        disable_session_recording: true,
        disable_surveys: true,
        advanced_disable_feature_flags: true,
        capture_heatmaps: false,
        capture_performance: false,
        disable_capture_url_hashes: true,
      })

      return posthog
    })
    .catch(() => null)

  return clientPromise
}

function currentPageUrl() {
  const url = new URL(window.location.href)
  url.search = ""
  url.hash = ""
  return url.toString()
}

export async function captureMarketingPageView() {
  const pageUrl = currentPageUrl()
  const client = await loadClient()
  client?.capture("$pageview", { $current_url: pageUrl })
}

export async function captureMarketingEvent(event: MarketingAnalyticsEvent) {
  const pageUrl = currentPageUrl()
  const client = await loadClient()
  client?.capture(event.name, {
    ...event.properties,
    $current_url: pageUrl,
  })
}
