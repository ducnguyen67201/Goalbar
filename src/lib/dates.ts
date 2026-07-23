export function relativeDate(value: string, now = Date.now()) {
  const elapsed = new Date(value).getTime() - now
  const formatter = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" })
  const minutes = Math.round(elapsed / 60_000)
  if (Math.abs(minutes) < 60) return formatter.format(minutes, "minute")
  const hours = Math.round(minutes / 60)
  if (Math.abs(hours) < 48) return formatter.format(hours, "hour")
  return formatter.format(Math.round(hours / 24), "day")
}
