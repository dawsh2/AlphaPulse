import React, { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null
    };
  }

  static getDerivedStateFromError(error: Error): State {
    // Update state so the next render will show the fallback UI
    return {
      hasError: true,
      error,
      errorInfo: null
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    // Log error to console in development
    if (process.env.NODE_ENV === 'development') {
      console.error('ErrorBoundary caught an error:', error, errorInfo);
    }
    
    // Update state with error details
    this.setState({
      error,
      errorInfo
    });

    // TODO: Send error to monitoring service in production
    // logErrorToService(error, errorInfo);
  }

  handleReset = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null
    });
  };

  render() {
    if (this.state.hasError) {
      // You can render any custom fallback UI
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div style={{
          padding: '20px',
          backgroundColor: '#1a1a1a',
          border: '1px solid #ff4444',
          borderRadius: '8px',
          color: '#ffffff',
          fontFamily: 'monospace'
        }}>
          <h2 style={{ color: '#ff4444', marginBottom: '10px' }}>
            Dashboard Error
          </h2>
          <p style={{ marginBottom: '10px' }}>
            Something went wrong in the dashboard. The error has been logged.
          </p>
          {process.env.NODE_ENV === 'development' && this.state.error && (
            <details style={{ marginTop: '10px' }}>
              <summary style={{ cursor: 'pointer', marginBottom: '10px' }}>
                Error Details (Development Only)
              </summary>
              <pre style={{ 
                backgroundColor: '#0a0a0a', 
                padding: '10px', 
                borderRadius: '4px',
                overflow: 'auto',
                fontSize: '12px'
              }}>
                {this.state.error.toString()}
                {this.state.errorInfo && this.state.errorInfo.componentStack}
              </pre>
            </details>
          )}
          <button
            onClick={this.handleReset}
            style={{
              marginTop: '15px',
              padding: '8px 16px',
              backgroundColor: '#4a4a4a',
              color: '#ffffff',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            Reset Dashboard
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}