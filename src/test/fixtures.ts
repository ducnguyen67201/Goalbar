import type { BootstrapState } from "@/schemas/bootstrap"
import type { HistoryOverview } from "@/schemas/history"

export const founderFixture = {
  id: "3d78d894-f753-4fc1-8697-d07e5a97a418",
  name: "Duc",
  productName: "Acme",
  offer: "A local sustainable growth system",
  expertise: "Building local-first products",
  goals: ["Qualified conversations"],
  boundaries: ["No spam"],
  onboardingCompleted: true,
  createdAt: "2026-07-22T00:00:00Z",
  updatedAt: "2026-07-22T00:00:00Z",
}

export const bootstrapFixture: BootstrapState = {
  schemaVersion: 1,
  founder: founderFixture,
  agents: [{ provider: "codex", readiness: "ready", version: "0.145.0" }],
  accounts: [],
  score: {
    formulaVersion: 1,
    score: 42,
    confidence: 0.5,
    components: { conversationQuality: 50, relationshipGrowth: 34 },
    missing: ["attentionQuality", "consistency", "learningVelocity"],
  },
  nextActions: [
    {
      kind: "experiment",
      title: "Run a focused experiment",
      reason: "Turn one insight into evidence.",
      route: "/create",
      priority: 70,
    },
  ],
}

export const historyOverviewFixture: HistoryOverview = {
  schemaVersion: 1,
  sourceCount: 2,
  itemCount: 128,
  platforms: [
    {
      platform: "x",
      sourceCount: 1,
      itemCount: 96,
      ownItemCount: 72,
      referenceItemCount: 24,
      latestAt: "2026-07-22T00:00:00Z",
    },
    {
      platform: "linkedin",
      sourceCount: 1,
      itemCount: 32,
      ownItemCount: 24,
      referenceItemCount: 8,
      latestAt: "2026-07-21T00:00:00Z",
    },
  ],
}
