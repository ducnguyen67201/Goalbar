/* global document, Event, HTMLInputElement, HTMLTextAreaElement, InputEvent, NodeFilter, window */
;(() => {
  const reply = String(globalThis.__GOALBAR_REPLY_TEXT__ || "")
  const platform = String(globalThis.__GOALBAR_REPLY_PLATFORM__ || "")
  delete globalThis.__GOALBAR_REPLY_TEXT__
  delete globalThis.__GOALBAR_REPLY_PLATFORM__
  const path = window.location.pathname.toLowerCase()

  if (/login|signin|signup|auth/.test(path)) {
    return JSON.stringify({ state: "login_required", characterCount: 0 })
  }
  if (/challenge|checkpoint|verify/.test(path)) {
    return JSON.stringify({ state: "verification_required", characterCount: 0 })
  }

  const roots = [document]
  const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_ELEMENT)
  let current = walker.currentNode
  while (current) {
    if (current.shadowRoot) roots.push(current.shadowRoot)
    current = walker.nextNode()
  }

  const query = (selectors) => {
    for (const selector of selectors) {
      for (const root of roots) {
        const node = root.querySelector(selector)
        if (!node) continue
        const rect = node.getBoundingClientRect()
        const style = window.getComputedStyle(node)
        if (rect.width > 0 && rect.height > 0 && style.visibility !== "hidden") return node
      }
    }
    return null
  }

  const composers = {
    x: [
      '[data-testid="tweetTextarea_0"][contenteditable="true"]',
      '[data-testid="tweetTextarea_0"] [contenteditable="true"]',
      '[role="textbox"][contenteditable="true"][aria-label*="reply" i]',
    ],
    linkedin: [
      '.comments-comment-box__form [contenteditable="true"]',
      '.comments-comment-box-comment__text-editor[contenteditable="true"]',
      '.comments-comment-box-comment__text-editor [contenteditable="true"]',
      '[role="textbox"][contenteditable="true"][aria-label*="comment" i]',
    ],
    reddit: [
      'textarea[name="text"]',
      'textarea[placeholder*="What are your thoughts" i]',
      'textarea[placeholder*="comment" i]',
      '[role="textbox"][contenteditable="true"][aria-label*="comment" i]',
      '[slot="rte"][contenteditable="true"]',
      'shreddit-composer [contenteditable="true"]',
    ],
  }

  const openers = {
    x: ['[data-testid="reply"]'],
    linkedin: [
      'button.comment-button:not([type="submit"])',
      'button[aria-label^="Comment on" i]:not([type="submit"])',
    ],
    reddit: [
      'button[aria-label="Add a comment" i]:not([type="submit"])',
      'button[aria-label="Join the conversation" i]:not([type="submit"])',
    ],
  }

  if (!composers[platform]) {
    return JSON.stringify({ state: "unsupported_page", characterCount: 0 })
  }

  let composer = query(composers[platform])
  if (!composer) {
    const opener = query(openers[platform])
    const unsafeOpener =
      opener?.matches?.('[type="submit"], [data-testid*="submit" i], [class*="submit" i]') ?? false
    if (opener && !unsafeOpener) {
      opener.click()
      return JSON.stringify({ state: "composer_opening", characterCount: 0 })
    }
    return JSON.stringify({ state: "composer_not_found", characterCount: 0 })
  }

  const nearestExcludedSurface = composer.closest?.(
    '[role="search"], [aria-label*="search" i], [aria-label*="message" i], [data-testid*="dmComposer" i]',
  )
  if (nearestExcludedSurface) {
    return JSON.stringify({ state: "composer_not_found", characterCount: 0 })
  }

  composer.focus()
  if (composer instanceof HTMLTextAreaElement || composer instanceof HTMLInputElement) {
    const prototype =
      composer instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype
    const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set
    if (setter) setter.call(composer, reply)
    else composer.value = reply
  } else {
    const selection = window.getSelection()
    const range = document.createRange()
    range.selectNodeContents(composer)
    selection?.removeAllRanges()
    selection?.addRange(range)
    const inserted = document.execCommand("insertText", false, reply)
    if (!inserted) composer.textContent = reply
    selection?.collapseToEnd()
  }

  try {
    composer.dispatchEvent(
      new InputEvent("input", {
        bubbles: true,
        composed: true,
        data: reply,
        inputType: "insertText",
      }),
    )
  } catch {
    composer.dispatchEvent(new Event("input", { bubbles: true, composed: true }))
  }
  composer.dispatchEvent(new Event("change", { bubbles: true }))
  composer.focus()
  composer.scrollIntoView({ block: "center", behavior: "smooth" })

  const values =
    composer instanceof HTMLTextAreaElement || composer instanceof HTMLInputElement
      ? [composer.value]
      : [composer.innerText || "", composer.textContent || ""]
  const value = values.find((candidate) => candidate === reply) || values[0]
  return JSON.stringify({
    state: values.includes(reply) ? "prepared" : "composer_not_found",
    characterCount: Array.from(value).length,
  })
})()
