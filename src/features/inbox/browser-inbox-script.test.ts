import { runInThisContext } from "node:vm"

import { beforeEach, describe, expect, it, vi } from "vitest"

import inboxScanScript from "../../../src-tauri/browser-scripts/inbox-scan.js?raw"

type ScanBatch = {
  state: string
  items: Array<{
    remoteId: string
    displayName: string
    preview: string
    unread: boolean
    remoteUrl: string
    profileUrl: string | null
    direction: string
  }>
}

function scan(
  platform: "x" | "reddit" | "linkedin",
  path: string,
  mode: "start" | "next" = "start",
): ScanBatch {
  window.history.replaceState({}, "", path)
  Object.assign(globalThis, {
    __GOALBAR_INBOX_SCAN_PLATFORM__: platform,
    __GOALBAR_INBOX_SCAN_MODE__: mode,
  })
  const executable = inboxScanScript.replace(";(() =>", "(() =>")
  return JSON.parse(runInThisContext(executable) as string) as ScanBatch
}

function setInnerText(selector: string, value: string) {
  Object.defineProperty(document.querySelector(selector), "innerText", {
    configurable: true,
    value,
  })
}

describe("browser inbox extraction script", () => {
  beforeEach(() => {
    document.body.innerHTML = ""
    document.body.addEventListener("click", (event) => event.preventDefault())
    delete (globalThis as Record<string, unknown>).__GOALBAR_INBOX_SCAN_STATE__
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      width: 420,
      height: 72,
      top: 0,
      right: 420,
      bottom: 72,
      left: 0,
      toJSON: () => ({}),
    })
  })

  it("extracts X conversation links, previews, direction, and unread state", () => {
    document.body.innerHTML = `
      <main>
        <div data-testid="conversation" aria-label="Unread conversation">
          <a href="https://x.com/messages/123">
            <strong>Ari</strong>
            <span>You: Thanks man</span>
            <time datetime="2026-07-17T18:00:00Z">6d</time>
          </a>
        </div>
      </main>
    `
    setInnerText('[data-testid="conversation"]', "Ari\nYou: Thanks man\n6d")

    const result = scan("x", "/messages")

    expect(result.state).toBe("ready")
    expect(result.items).toEqual([
      expect.objectContaining({
        remoteId: "messages/123",
        displayName: "Ari",
        preview: "You: Thanks man",
        unread: true,
        remoteUrl: "https://x.com/messages/123",
        direction: "outbound",
      }),
    ])
  })

  it.each([
    {
      platform: "linkedin" as const,
      path: "/messaging/",
      row: `<li class="msg-conversation-listitem">
        <a href="https://www.linkedin.com/messaging/thread/abc/">
          <strong>Mina</strong><span>Can we compare notes?</span>
        </a>
      </li>`,
      selector: ".msg-conversation-listitem",
      text: "Mina\nCan we compare notes?\n2h",
      remoteId: "messaging/thread/abc",
    },
    {
      platform: "reddit" as const,
      path: "/message/inbox",
      row: `<article class="Message">
        <a href="https://www.reddit.com/message/messages/xyz">
          <strong>u/founder</strong><span>Quick question</span>
        </a>
      </article>`,
      selector: ".Message",
      text: "u/founder\nQuick question\n1d",
      remoteId: "message/messages/xyz",
    },
  ])("extracts $platform conversation rows", ({ platform, path, row, selector, text, remoteId }) => {
    document.body.innerHTML = `<main>${row}</main>`
    setInnerText(selector, text)

    const result = scan(platform, path)

    expect(result.state).toBe("ready")
    expect(result.items[0]).toEqual(
      expect.objectContaining({
        remoteId,
        unread: false,
        direction: "inbound",
      }),
    )
  })

  it("removes LinkedIn's trailing undefined segment and keeps each direct thread URL", () => {
    document.body.innerHTML = `
      <main>
        <li id="ember47" class="msg-conversation-listitem">
          <a href="https://www.linkedin.com/messaging/thread/2-mailbox/undefined/">
            <strong>Ross McIntyre</strong><span>Status is reachable</span>
          </a>
        </li>
        <li id="ember55" class="msg-conversation-listitem">
          <a href="https://www.linkedin.com/messaging/thread/3-mailbox/undefined/">
            <strong>Sashank Tadepalli</strong><span>Status is reachable</span>
          </a>
        </li>
      </main>
    `
    setInnerText("#ember47", "Ross McIntyre\nStatus is reachable\nNow")
    setInnerText("#ember55", "Sashank Tadepalli\nStatus is reachable\nNow")

    const result = scan("linkedin", "/messaging/")

    expect(result.items).toEqual([
      expect.objectContaining({
        remoteId: "messaging/thread/2-mailbox",
        remoteUrl: "https://www.linkedin.com/messaging/thread/2-mailbox/",
      }),
      expect.objectContaining({
        remoteId: "messaging/thread/3-mailbox",
        remoteUrl: "https://www.linkedin.com/messaging/thread/3-mailbox/",
      }),
    ])
  })

  it("extracts a LinkedIn profile URL separately from the conversation URL", () => {
    document.body.innerHTML = `
      <main>
        <li class="msg-conversation-listitem">
          <a class="msg-conversation-listitem__participant-names" href="https://www.linkedin.com/in/vy-nguyen/?trk=messaging">
            <strong>Vy Nguyen</strong>
          </a>
          <a href="https://www.linkedin.com/messaging/thread/abc/">
            <span>Status is reachable</span>
          </a>
        </li>
      </main>
    `
    setInnerText(".msg-conversation-listitem", "Vy Nguyen\nStatus is reachable\nNow")

    const result = scan("linkedin", "/messaging/")

    expect(result.items[0]).toEqual(
      expect.objectContaining({
        remoteUrl: "https://www.linkedin.com/messaging/thread/abc/",
        profileUrl: "https://www.linkedin.com/in/vy-nguyen/",
      }),
    )
  })

  it("opens a LinkedIn row once and captures the profile exposed by the thread header", () => {
    document.body.innerHTML = `
      <main>
        <li id="vy" class="msg-conversation-listitem">
          <button type="button">
            <strong>Vy Nguyen</strong><span>Status is reachable</span>
          </button>
        </li>
      </main>
    `
    setInnerText("#vy", "Vy Nguyen\nStatus is reachable\nNow")
    document.querySelector("#vy")?.addEventListener("click", () => {
      document.body.insertAdjacentHTML(
        "beforeend",
        `<section class="msg-thread">
          <a
            class="msg-thread__link-to-profile"
            aria-label="View profile"
            href="https://www.linkedin.com/in/vy-nguyen/overlay/contact-info/?trk=messaging"
          ></a>
        </section>`,
      )
    })

    const first = scan("linkedin", "/messaging/")
    const second = scan("linkedin", "/messaging/", "next")

    expect(first.items[0]?.profileUrl).toBeNull()
    expect(second.items[0]?.profileUrl).toBe("https://www.linkedin.com/in/vy-nguyen/")
  })
})
