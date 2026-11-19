import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = { children: ReactNode; fallbackTitle?: string };
type State = { error: Error | null };

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("CodeSetu UI fault:", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="fallback-panel" role="alert">
          <h2>{this.props.fallbackTitle ?? "Something went wrong"}</h2>
          <p>
            The panel crashed, but CodeSetu is still running. Your folders were
            not modified by this error.
          </p>
          <p className="muted tiny">
            Recovery: try again, or return to Folders and rescan. If it repeats,
            use Settings → Pilot tools → Export diagnostic report and share it
            with your pilot contact (local file only).
          </p>
          <pre className="fallback-detail">{this.state.error.message}</pre>
          <button
            type="button"
            className="btn primary"
            onClick={() => this.setState({ error: null })}
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
