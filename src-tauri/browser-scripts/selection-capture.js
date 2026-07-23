/* global window */
/* eslint-disable no-control-regex */
;(() => {
  const value = String(window.getSelection?.()?.toString() ?? "")
    .replace(/[\u0000-\u001f\u007f]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 20000)
  return JSON.stringify({ selectedText: value || null })
})()
