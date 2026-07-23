import { engagementSuggestionSchema, type EngagementSuggestion } from "@/schemas/agent"

const taggedSuggestionPattern = /<goalbar-engagement>\s*([\s\S]*?)\s*<\/goalbar-engagement>/i
const urlPattern = /https?:\/\/[^\s<>"']+/gi

function cleanMarkdown(value: string) {
  return value
    .replace(/^\s*>\s?/gm, "")
    .replace(/\*\*/g, "")
    .replace(/^\s*[-–—]\s*/, "")
    .trim()
}

function trimUrl(value: string) {
  return value.replace(/[)\],.!?]+$/g, "")
}

function fallbackTitle(url: string) {
  try {
    return `Post on ${new URL(url).hostname.replace(/^www\./, "")}`
  } catch {
    return "Recommended post"
  }
}

export function parseEngagementSuggestion(body: string): EngagementSuggestion | null {
  const tagged = body.match(taggedSuggestionPattern)
  if (tagged) {
    try {
      const parsed = engagementSuggestionSchema.safeParse(JSON.parse(tagged[1]))
      if (parsed.success) return parsed.data
    } catch {
      // Fall through so older, human-readable Goalbar replies still become action cards.
    }
  }

  const replyHeading = body.match(/\*{0,2}\s*Suggested reply:\s*\*{0,2}/i)
  if (replyHeading?.index === undefined) return null

  const recommendation = body.slice(0, replyHeading.index)
  const urlMatches = [...recommendation.matchAll(urlPattern)]
  const urlMatch = urlMatches.at(-1)
  if (!urlMatch?.[0] || urlMatch.index === undefined) return null

  const url = trimUrl(urlMatch[0])
  const bracketedTitles = [...recommendation.matchAll(/\[([^\]\n]{2,300})\]/g)]
  const headingTitle = recommendation.match(
    /(?:Best post to engage with|Recommended post):\s*\*{0,2}\s*([^\n[]+)/i,
  )?.[1]
  const title = cleanMarkdown(bracketedTitles.at(-1)?.[1] ?? headingTitle ?? fallbackTitle(url))

  const reason = cleanMarkdown(
    recommendation.slice(urlMatch.index + urlMatch[0].length).replace(/^[\s)\]]+/, ""),
  )
  const replyStart = replyHeading.index + replyHeading[0].length
  let replySection = body.slice(replyStart).trim()
  const disclaimer = replySection.search(
    /\n{2,}\s*(?:I (?:can['’]?t|cannot)\b|Nothing (?:was|is)\b|You(?:['’]ll| can)\b)/i,
  )
  if (disclaimer >= 0) replySection = replySection.slice(0, disclaimer)

  const parsed = engagementSuggestionSchema.safeParse({
    title,
    url,
    reason: reason || "This post is a strong audience fit with room for a thoughtful response.",
    reply: cleanMarkdown(replySection),
  })
  return parsed.success ? parsed.data : null
}
