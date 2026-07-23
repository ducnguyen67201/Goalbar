import { Linkedin, MessageCircle, ShieldCheck } from "lucide-react"

const platforms = [
  {
    id: "x",
    name: "X",
    description: "Posts, conversations, and founder accounts",
    url: "https://x.com/home",
    icon: <span className="browser-platform-letter">X</span>,
  },
  {
    id: "linkedin",
    name: "LinkedIn",
    description: "Professional feeds, profiles, and ICP research",
    url: "https://www.linkedin.com/feed/",
    icon: <Linkedin size={22} />,
  },
  {
    id: "reddit",
    name: "Reddit",
    description: "Communities, customer language, and pain points",
    url: "https://www.reddit.com/",
    icon: <MessageCircle size={22} />,
  },
] as const

type BrowserStartPageProps = {
  onOpen: (url: string) => Promise<void>
}

export function BrowserStartPage({ onOpen }: BrowserStartPageProps) {
  return (
    <div className="browser-start-page">
      <div className="browser-start-eyebrow">
        <ShieldCheck size={14} />
        Local browser session
      </div>
      <h2>Where do you want to research?</h2>
      <p>Choose a platform. Your login and website session stay on this machine.</p>
      <div className="browser-platform-grid">
        {platforms.map((platform) => (
          <button key={platform.id} onClick={() => void onOpen(platform.url)}>
            <span className={`browser-platform-icon ${platform.id}`}>{platform.icon}</span>
            <span>
              <strong>Open {platform.name}</strong>
              <small>{platform.description}</small>
            </span>
          </button>
        ))}
      </div>
    </div>
  )
}
