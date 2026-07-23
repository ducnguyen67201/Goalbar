import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi } from "vitest"

import { EngagementSuggestionCard } from "./EngagementSuggestionCard"
import { parseEngagementSuggestion } from "./engagement-suggestion"

const suggestion = {
  title: "Julia Fedorin on founders’ untold origin stories",
  url: "https://x.com/juliafedorin/status/2079974598108746156",
  reason: "It is highly relevant to founder voice and has room for a thoughtful response.",
  reply:
    "The most compelling founder stories usually start before the startup—an obsession, frustration, or failed attempt that only makes sense in hindsight.",
}

describe("parseEngagementSuggestion", () => {
  it("parses the machine-readable agent format", () => {
    const result = parseEngagementSuggestion(`
I found one strong fit.
<goalbar-engagement>
${JSON.stringify(suggestion)}
</goalbar-engagement>
    `)

    expect(result).toEqual(suggestion)
  })

  it("does not turn unsupported URLs into clickable actions", () => {
    const result = parseEngagementSuggestion(`
<goalbar-engagement>
${JSON.stringify({ ...suggestion, url: "https://example.com/fake-post" })}
</goalbar-engagement>
    `)

    expect(result).toBeNull()
  })

  it("upgrades the existing human-readable recommendation into an action card", () => {
    const result = parseEngagementSuggestion(`
**Best post to engage with:** [Julia Fedorin on founders’ untold origin stories]
(https://x.com/juliafedorin/status/2079974598108746156)

It’s highly relevant to founder voice and has only three replies—good room for a thoughtful response.

**Suggested reply:**

> The most compelling founder stories usually start before the startup—an obsession, frustration, or failed attempt that only makes sense in hindsight.

I can’t post it for you; paste the reply on X if it fits your voice.
    `)

    expect(result).toMatchObject({
      title: suggestion.title,
      url: suggestion.url,
      reply: suggestion.reply,
    })
    expect(result?.reason).toContain("highly relevant")
  })
})

describe("EngagementSuggestionCard", () => {
  it("reviews the exact reply before filling the browser composer without submitting it", async () => {
    const user = userEvent.setup()
    const onOpen = vi.fn()
    const onPrepare = vi.fn().mockResolvedValue({
      status: "prepared",
      platform: "x",
      characterCount: 23,
      savedReply: {
        id: "c0d9a1c8-2198-43f1-bde8-c63cd3be7e9a",
        platform: "x",
        targetUrl: suggestion.url,
        exactReply: "This is my exact reply.",
        status: "prepared",
        preparedAt: "2026-07-23T18:00:00Z",
        confirmedPostedAt: null,
      },
    })
    const onRewrite = vi.fn()

    render(
      <EngagementSuggestionCard
        suggestion={suggestion}
        rewritePending={false}
        onOpen={onOpen}
        onPrepare={onPrepare}
        onRewrite={onRewrite}
      />,
    )

    await user.click(screen.getByRole("button", { name: "Comment" }))
    const exactReply = screen.getByRole("textbox", { name: "Exact reply" })
    await user.clear(exactReply)
    await user.type(exactReply, "This is my exact reply.")
    await user.click(screen.getByRole("button", { name: "Put in browser" }))

    await waitFor(() => expect(onPrepare).toHaveBeenCalledWith(suggestion.url, "This is my exact reply."))
    expect(onOpen).not.toHaveBeenCalled()
    expect(
      screen.getByText(
        "Ready in the browser and saved locally with this exact text. It is not marked as posted; review it there, then click Comment or Reply yourself.",
      ),
    ).toBeVisible()
  })

  it("sends the current edited revision to rewrite", async () => {
    const user = userEvent.setup()
    const onRewrite = vi.fn()

    render(
      <EngagementSuggestionCard
        suggestion={suggestion}
        rewritePending={false}
        onOpen={() => undefined}
        onPrepare={vi.fn()}
        onRewrite={onRewrite}
      />,
    )

    await user.click(screen.getByRole("button", { name: "Comment" }))
    const exactReply = screen.getByRole("textbox", { name: "Exact reply" })
    await user.clear(exactReply)
    await user.type(exactReply, "Make this warmer.")
    await user.click(screen.getByRole("button", { name: "Rewrite" }))

    expect(onRewrite).toHaveBeenCalledWith({
      ...suggestion,
      reply: "Make this warmer.",
    })
  })
})
