import { beforeEach, describe, expect, it, vi } from "vitest"

const posthog = vi.hoisted(() => ({
  capture: vi.fn(),
  init: vi.fn(),
}))

vi.mock("posthog-js", () => ({ default: posthog }))

describe("marketing analytics", () => {
  beforeEach(() => {
    vi.resetModules()
    vi.unstubAllEnvs()
    posthog.capture.mockReset()
    posthog.init.mockReset()
    window.history.replaceState({}, "", "/")
  })

  it("stays disabled when the marketing project key is absent", async () => {
    const { captureMarketingPageView } = await import("./marketing-analytics")

    await captureMarketingPageView()

    expect(posthog.init).not.toHaveBeenCalled()
    expect(posthog.capture).not.toHaveBeenCalled()
  })

  it("captures an anonymous pageview without URL query data", async () => {
    vi.stubEnv("VITE_POSTHOG_KEY", "phc_test")
    vi.stubEnv("VITE_POSTHOG_HOST", "https://us.i.posthog.com")
    window.history.replaceState({}, "", "/landing?email=private%40example.com#download")
    const { captureMarketingPageView } = await import("./marketing-analytics")

    await captureMarketingPageView()

    expect(posthog.init).toHaveBeenCalledWith(
      "phc_test",
      expect.objectContaining({
        api_host: "https://us.i.posthog.com",
        autocapture: false,
        disable_session_recording: true,
        persistence: "memory",
        person_profiles: "identified_only",
      }),
    )
    expect(posthog.capture).toHaveBeenCalledWith("$pageview", {
      $current_url: "http://localhost:3000/landing",
    })
  })

  it("captures download intent without the submitted email address", async () => {
    vi.stubEnv("VITE_POSTHOG_KEY", "phc_test")
    const { captureMarketingEvent } = await import("./marketing-analytics")

    await captureMarketingEvent({
      name: "marketing_download_requested",
      properties: { placement: "hero" },
    })

    expect(posthog.capture).toHaveBeenCalledWith("marketing_download_requested", {
      placement: "hero",
      $current_url: "http://localhost:3000/",
    })
  })
})
