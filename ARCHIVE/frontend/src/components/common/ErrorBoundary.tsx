import React from 'react';

interface ErrorBoundaryProps {
  children: React.ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('Sumi UI runtime error', error, info);
  }

  private handleRetry = () => {
    this.setState({ error: null });
  };

  private handleReload = () => {
    window.location.reload();
  };

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <div className="glass-panel" role="alert" style={{ padding: '2rem', maxWidth: '720px' }}>
        <h2 style={{ marginTop: 0 }}>Something went wrong</h2>
        <p style={{ color: 'var(--text-muted)', lineHeight: 1.6 }}>
          This view could not render. Your local data is still safe.
        </p>
        <pre style={{
          whiteSpace: 'pre-wrap',
          background: 'rgba(0,0,0,0.25)',
          border: '1px solid var(--border-color)',
          borderRadius: '6px',
          color: 'var(--text-muted)',
          padding: '12px',
          maxHeight: '160px',
          overflow: 'auto',
        }}>
          {this.state.error.message}
        </pre>
        <div style={{ display: 'flex', gap: '12px', marginTop: '16px' }}>
          <button className="btn-primary" type="button" onClick={this.handleRetry}>
            Retry
          </button>
          <button type="button" onClick={this.handleReload}>
            Reload
          </button>
        </div>
      </div>
    );
  }
}
