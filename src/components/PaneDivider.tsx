type PaneDividerProps = {
  onMove: (delta: number) => void
  label: string
}

export function PaneDivider({ onMove, label }: PaneDividerProps) {
  return (
    <div
      className="pane-divider"
      role="separator"
      aria-label={label}
      aria-orientation="vertical"
      tabIndex={0}
      onPointerDown={(event) => {
        let previous = event.clientX
        const target = event.currentTarget
        target.setPointerCapture(event.pointerId)
        const move = (moveEvent: PointerEvent) => {
          onMove(moveEvent.clientX - previous)
          previous = moveEvent.clientX
        }
        const stop = () => {
          target.removeEventListener("pointermove", move)
          target.removeEventListener("pointerup", stop)
          target.removeEventListener("pointercancel", stop)
        }
        target.addEventListener("pointermove", move)
        target.addEventListener("pointerup", stop)
        target.addEventListener("pointercancel", stop)
      }}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") onMove(-16)
        if (event.key === "ArrowRight") onMove(16)
      }}
    >
      <span />
    </div>
  )
}
