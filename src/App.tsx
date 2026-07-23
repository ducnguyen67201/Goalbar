import { Navigate, Route, Routes } from "react-router-dom"

import { AppShell } from "@/components/AppShell"
import { CreatePage } from "@/features/create/CreatePage"
import { GrowthPage } from "@/features/growth/GrowthPage"
import { InboxPage } from "@/features/inbox/InboxPage"
import { MemoryPage } from "@/features/memory/MemoryPage"
import { OnboardingFlow } from "@/features/onboarding/OnboardingFlow"
import { SettingsPage } from "@/features/settings/SettingsPage"
import { TodayPage } from "@/features/today/TodayPage"

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<TodayPage />} />
        <Route path="onboarding" element={<OnboardingFlow />} />
        <Route path="create" element={<CreatePage />} />
        <Route path="inbox" element={<InboxPage />} />
        <Route path="growth" element={<GrowthPage />} />
        <Route path="memory" element={<MemoryPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}
