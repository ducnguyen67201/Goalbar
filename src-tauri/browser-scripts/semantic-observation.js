/* global document, URL, window */
/* eslint-disable no-control-regex */
;(() => {
  const normalize = (value) =>
    String(value ?? "")
      .replace(/[\u0000-\u001f\u007f]+/g, " ")
      .replace(/\s+/g, " ")
      .trim()
  const canonical = (value) => {
    try {
      const url = new URL(value, window.location.href)
      if (url.protocol !== "https:") return null
      url.search = ""
      url.hash = ""
      return url.toString()
    } catch {
      return null
    }
  }
  const candidates = Array.from(document.querySelectorAll("article, [role='article'], main section, main li"))
    .filter((node) => {
      const rect = node.getBoundingClientRect()
      return rect.bottom >= 0 && rect.top <= window.innerHeight && rect.width > 0 && rect.height > 0
    })
    .slice(0, 40)
  const blocks = candidates
    .map((node, index) => {
      const text = normalize(node.innerText).slice(0, 4000)
      const links = Array.from(node.querySelectorAll("a[href]"))
        .map((link) => canonical(link.href))
        .filter(Boolean)
        .slice(0, 12)
      const timestamp = node.querySelector("time")?.getAttribute("datetime") ?? null
      return {
        key: `${index}:${text.slice(0, 80)}`,
        role: normalize(node.getAttribute("role") || node.tagName.toLowerCase()),
        text,
        links,
        timestamp,
      }
    })
    .filter((block) => block.text.length > 0)
  return JSON.stringify({
    title: normalize(document.title).slice(0, 160),
    viewport: {
      width: Math.max(0, Math.round(window.innerWidth)),
      height: Math.max(0, Math.round(window.innerHeight)),
      scrollY: Math.max(0, window.scrollY),
    },
    blocks,
  })
})()
