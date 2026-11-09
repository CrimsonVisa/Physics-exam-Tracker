import { Component, type ReactNode } from 'react';

type FormulaRouteErrorBoundaryProps = {
  children: ReactNode;
  onBack: () => void;
  title?: string;
  message?: string;
};

type FormulaRouteErrorBoundaryState = {
  hasError: boolean;
};

export class FormulaRouteErrorBoundary extends Component<
  FormulaRouteErrorBoundaryProps,
  FormulaRouteErrorBoundaryState
> {
  state: FormulaRouteErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): FormulaRouteErrorBoundaryState {
    return { hasError: true };
  }

  render() {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <div className="page-container">
        <section className="v2-empty-panel" role="alert">
          <h1>{this.props.title ?? 'Formula reference unavailable'}</h1>
          <p>
            {this.props.message ??
              'The formula reference could not be loaded. This does not affect your saved study data.'}
          </p>
          <button className="secondary-button" onClick={this.props.onBack} type="button">
            Return to dashboard
          </button>
        </section>
      </div>
    );
  }
}
