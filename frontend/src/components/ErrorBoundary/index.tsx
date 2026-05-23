import React from 'react';
import { Result, Button } from 'antd';

interface State {
  hasError: boolean;
  error?: Error;
}

interface Props {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

/** Page-level error boundary — catches unhandled render errors and shows a recovery UI. */
class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    // In production, report to your monitoring service here
    console.error('[ErrorBoundary]', error, info.componentStack);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: undefined });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            minHeight: 400,
          }}
        >
          <Result
            status="error"
            title="Something went wrong"
            subTitle={
              this.state.error?.message ?? 'An unexpected error occurred in this page section.'
            }
            extra={[
              <Button
                key="retry"
                type="primary"
                onClick={this.handleReset}
                style={{ background: '#e05d10', borderColor: '#e05d10' }}
              >
                Try Again
              </Button>,
              <Button key="reload" onClick={() => window.location.reload()}>
                Reload Page
              </Button>,
            ]}
            style={{ background: 'transparent' }}
          />
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
