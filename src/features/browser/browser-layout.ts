const MIN_CONTROL_PANEL_WIDTH = 280
const MAX_CONTROL_PANEL_WIDTH = 480
const MIN_BROWSER_SURFACE_WIDTH = 420
const PANE_DIVIDER_WIDTH = 8

export function responsiveBrowserPanelWidth(preferredWidth: number, workspaceWidth: number): number {
  const preferred = Math.max(MIN_CONTROL_PANEL_WIDTH, Math.min(MAX_CONTROL_PANEL_WIDTH, preferredWidth))
  if (workspaceWidth <= 0) return preferred

  const viewportMaximum = workspaceWidth - MIN_BROWSER_SURFACE_WIDTH - PANE_DIVIDER_WIDTH
  return Math.max(MIN_CONTROL_PANEL_WIDTH, Math.min(preferred, viewportMaximum))
}

export function clampBrowserPanelWidth(width: number): number {
  return Math.max(MIN_CONTROL_PANEL_WIDTH, Math.min(MAX_CONTROL_PANEL_WIDTH, width))
}
