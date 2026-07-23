import { Component, type ReactNode } from "react"

interface State {
  error: Error | null
}

export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidCatch(error: Error) {
    console.error("[tagline] render failure", error.message)
  }

  render() {
    if (!this.state.error) return this.props.children
    return (
      <main className="fatal-error">
        <p className="eyebrow">Local UI error</p>
        <h1>The lab hit a snag.</h1>
        <p>{this.state.error.message}</p>
        <button className="button button-primary" onClick={() => this.setState({ error: null })}>
          Try again
        </button>
      </main>
    )
  }
}
