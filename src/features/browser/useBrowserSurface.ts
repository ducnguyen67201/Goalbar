import { listen } from "@tauri-apps/api/event"
import { useCallback, useEffect, useRef, useState } from "react"
import { z } from "zod"

import { invokeOutput, invokeValidated, isTauriRuntime } from "@/lib/tauri"
import {
  browserBoundsInputSchema,
  browserNewWindowRequestSchema,
  browserReplyPreparationSchema,
  browserTabInputSchema,
  browserTabSchema,
  createBrowserTabInputSchema,
  navigateBrowserInputSchema,
  prepareBrowserReplyInputSchema,
  type BrowserBounds,
  type BrowserReplyPreparation,
  type BrowserTab,
} from "@/schemas/browser"

export function deduplicateBrowserTabs(tabs: BrowserTab[]): BrowserTab[] {
  const tabsById = new Map<string, BrowserTab>()
  for (const tab of tabs) tabsById.set(tab.id, tab)
  return [...tabsById.values()]
}

export function upsertBrowserTab(tabs: BrowserTab[], updated: BrowserTab): BrowserTab[] {
  const uniqueTabs = deduplicateBrowserTabs(tabs)
  const existingIndex = uniqueTabs.findIndex((tab) => tab.id === updated.id)
  const nextTabs =
    existingIndex === -1
      ? [...uniqueTabs, updated]
      : uniqueTabs.map((tab, index) => (index === existingIndex ? updated : tab))

  return updated.active
    ? nextTabs.map((tab) => (tab.id === updated.id ? tab : { ...tab, active: false }))
    : nextTabs
}

const previewTab: BrowserTab = {
  id: "b9d7afe0-1807-4ad9-bf22-2945f0bb9081",
  webviewLabel: "browser-preview",
  currentUrl: "https://x.com/home",
  title: "X home",
  loadState: "loaded",
  platform: "x",
  active: true,
  createdAt: new Date().toISOString(),
}

export function useBrowserSurface() {
  const surfaceRef = useRef<HTMLDivElement>(null)
  const [tabs, setTabs] = useState<BrowserTab[]>(isTauriRuntime() ? [] : [previewTab])
  const [error, setError] = useState<string | null>(null)
  const [newWindowUrl, setNewWindowUrl] = useState<string | null>(null)
  const [startPageOpen, setStartPageOpen] = useState(true)
  const visibleTabs = deduplicateBrowserTabs(tabs)
  const activeTab = startPageOpen ? null : (visibleTabs.find((tab) => tab.active) ?? null)

  const refresh = useCallback(async (): Promise<BrowserTab[]> => {
    if (!isTauriRuntime()) return []
    const value = await invokeOutput("list_browser_tabs", {}, z.array(browserTabSchema))
    setTabs(value)
    return value
  }, [])

  const bounds = useCallback((): BrowserBounds | null => {
    const rect = surfaceRef.current?.getBoundingClientRect()
    if (!rect || rect.width < 80 || rect.height < 80) return null
    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
  }, [])

  const createTab = useCallback(
    async (url: string): Promise<BrowserTab | null> => {
      if (!isTauriRuntime()) {
        const created = {
          ...previewTab,
          id: crypto.randomUUID(),
          currentUrl: url,
          title: "New tab",
          active: true,
        }
        setTabs((current) => upsertBrowserTab(current, created))
        setStartPageOpen(false)
        return created
      }
      const currentBounds = bounds()
      if (!currentBounds) return null
      const input = { url, bounds: currentBounds }
      const created = await invokeValidated(
        "create_browser_tab",
        { input },
        createBrowserTabInputSchema,
        browserTabSchema,
      )
      setTabs((current) => upsertBrowserTab(current, created))
      setStartPageOpen(false)
      return created
    },
    [bounds],
  )

  useEffect(() => {
    if (!isTauriRuntime()) return
    void invokeOutput("list_browser_tabs", {}, z.array(browserTabSchema))
      .then(setTabs)
      .catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)))
    const unlistenTab = listen<unknown>("browser://tab-updated", (event) => {
      const updated = browserTabSchema.parse(event.payload)
      setTabs((current) => upsertBrowserTab(current, updated))
    })
    const unlistenNewWindow = listen<unknown>("browser://new-window-requested", (event) => {
      const request = browserNewWindowRequestSchema.parse(event.payload)
      setNewWindowUrl(request.url)
    })
    return () => {
      void unlistenTab.then((unlisten) => unlisten())
      void unlistenNewWindow.then((unlisten) => unlisten())
      void invokeOutput("hide_browser_views", {}, z.boolean()).catch(() => undefined)
    }
  }, [])

  useEffect(() => {
    const node = surfaceRef.current
    if (!node || typeof ResizeObserver === "undefined") return
    let frame = 0
    const syncBounds = () => {
      if (!isTauriRuntime()) return
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => {
        const currentBounds = bounds()
        if (!currentBounds) return
        const input = { bounds: currentBounds }
        void invokeValidated("update_browser_bounds", { input }, browserBoundsInputSchema, z.boolean()).catch(
          (reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)),
        )
      })
    }
    const observer = new ResizeObserver(syncBounds)
    observer.observe(node)
    window.addEventListener("resize", syncBounds)
    window.addEventListener("scroll", syncBounds, true)
    return () => {
      cancelAnimationFrame(frame)
      observer.disconnect()
      window.removeEventListener("resize", syncBounds)
      window.removeEventListener("scroll", syncBounds, true)
    }
  }, [bounds])

  useEffect(() => {
    if (!isTauriRuntime() || !startPageOpen) return
    void invokeOutput("hide_browser_views", {}, z.boolean()).catch((reason: unknown) =>
      setError(reason instanceof Error ? reason.message : String(reason)),
    )
  }, [startPageOpen])

  const activate = async (tabId: string) => {
    if (!isTauriRuntime()) {
      setTabs((current) => current.map((tab) => ({ ...tab, active: tab.id === tabId })))
      setStartPageOpen(false)
      return
    }
    const input = { tabId }
    const tab = await invokeValidated(
      "activate_browser_tab",
      { input },
      browserTabInputSchema,
      browserTabSchema,
    )
    setTabs((current) => current.map((item) => ({ ...item, active: item.id === tab.id })))
    setStartPageOpen(false)
  }

  const navigate = async (url: string) => {
    if (!activeTab) {
      await createTab(url)
      return
    }
    if (!isTauriRuntime()) {
      setTabs((current) =>
        current.map((tab) => (tab.id === activeTab.id ? { ...tab, currentUrl: url } : tab)),
      )
      return
    }
    const input = { tabId: activeTab.id, url }
    const tab = await invokeValidated(
      "navigate_browser_tab",
      { input },
      navigateBrowserInputSchema,
      browserTabSchema,
    )
    setTabs((current) => current.map((item) => (item.id === tab.id ? tab : item)))
  }

  const openUrlInPlatform = useCallback(
    async (url: string, platform: NonNullable<BrowserTab["platform"]>): Promise<BrowserTab | null> => {
      if (!isTauriRuntime()) {
        const matchingTab = visibleTabs.find((tab) => tab.platform === platform)
        if (!matchingTab) return createTab(url)
        const updated = { ...matchingTab, currentUrl: url, active: true }
        setTabs((current) => upsertBrowserTab(current, updated))
        setStartPageOpen(false)
        return updated
      }

      const currentBounds = bounds()
      if (!currentBounds) return null
      const boundsInput = { bounds: currentBounds }
      await invokeValidated(
        "update_browser_bounds",
        { input: boundsInput },
        browserBoundsInputSchema,
        z.boolean(),
      )

      const currentTabs = await refresh()
      const matchingTab = currentTabs.find((tab) => tab.platform === platform)
      if (!matchingTab) return createTab(url)

      const tabInput = { tabId: matchingTab.id }
      await invokeValidated(
        "activate_browser_tab",
        { input: tabInput },
        browserTabInputSchema,
        browserTabSchema,
      )
      const navigateInput = { tabId: matchingTab.id, url }
      const navigated = await invokeValidated(
        "navigate_browser_tab",
        { input: navigateInput },
        navigateBrowserInputSchema,
        browserTabSchema,
      )
      setTabs((current) => upsertBrowserTab(current, navigated))
      setStartPageOpen(false)
      return navigated
    },
    [bounds, createTab, refresh, visibleTabs],
  )

  const prepareReply = async (url: string, reply: string): Promise<BrowserReplyPreparation> => {
    if (!isTauriRuntime()) {
      await navigate(url)
      return browserReplyPreparationSchema.parse({
        status: "unsupported_page",
        platform: activeTab?.platform ?? null,
        characterCount: 0,
      })
    }

    const targetTab = activeTab ?? (await createTab(url))
    if (!targetTab) {
      return browserReplyPreparationSchema.parse({
        status: "composer_not_found",
        platform: null,
        characterCount: 0,
      })
    }
    const input = { tabId: targetTab.id, url, reply }
    return invokeValidated(
      "prepare_browser_reply",
      { input },
      prepareBrowserReplyInputSchema,
      browserReplyPreparationSchema,
    )
  }

  const simpleAction = async (command: string) => {
    if (!activeTab || !isTauriRuntime()) return
    const input = { tabId: activeTab.id }
    await invokeValidated(command, { input }, browserTabInputSchema, z.boolean())
  }

  const close = async (tabId: string) => {
    if (!isTauriRuntime()) {
      const remainingTabs = visibleTabs.filter((tab) => tab.id !== tabId)
      setTabs(
        remainingTabs.map((tab, index) => ({
          ...tab,
          active: index === remainingTabs.length - 1,
        })),
      )
      setStartPageOpen(remainingTabs.length === 0)
      return
    }
    const input = { tabId }
    await invokeValidated("close_browser_tab", { input }, browserTabInputSchema, z.boolean())
    const remainingTabs = await refresh()
    if (remainingTabs.length === 0) {
      setStartPageOpen(true)
      return
    }
    if (!remainingTabs.some((tab) => tab.active)) {
      await activate(remainingTabs.at(-1)!.id)
    }
  }

  const openStartPage = async () => {
    if (isTauriRuntime()) {
      await invokeOutput("hide_browser_views", {}, z.boolean())
    }
    setTabs((current) => current.map((tab) => ({ ...tab, active: false })))
    setStartPageOpen(true)
  }

  const closeStartPage = async () => {
    const fallbackTab = visibleTabs.at(-1)
    if (fallbackTab) await activate(fallbackTab.id)
  }

  return {
    surfaceRef,
    tabs: visibleTabs,
    activeTab,
    startPageOpen,
    error,
    newWindowUrl,
    dismissNewWindow: () => setNewWindowUrl(null),
    openNewWindow: async () => {
      if (!newWindowUrl) return
      await createTab(newWindowUrl)
      setNewWindowUrl(null)
    },
    isNative: isTauriRuntime(),
    createTab,
    openStartPage,
    closeStartPage,
    activate,
    navigate,
    openUrlInPlatform,
    prepareReply,
    close,
    back: () => simpleAction("browser_go_back"),
    forward: () => simpleAction("browser_go_forward"),
    reload: () => simpleAction("reload_browser_tab"),
  }
}
