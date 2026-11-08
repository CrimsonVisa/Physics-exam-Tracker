import { lazy, Suspense } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FormulaRouteErrorBoundary } from './FormulaRouteErrorBoundary';

describe('FormulaRouteErrorBoundary', () => {
  it('handles a lazy chunk failure and returns to the dashboard', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const onBack = vi.fn();
    const FailingRoute = lazy(() => Promise.reject(new Error('formula chunk failed')));

    try {
      render(
        <FormulaRouteErrorBoundary onBack={onBack}>
          <Suspense fallback={<p>Loading formula reference…</p>}>
            <FailingRoute />
          </Suspense>
        </FormulaRouteErrorBoundary>,
      );

      expect(await screen.findByRole('alert')).toHaveTextContent('Formula reference unavailable');
      await userEvent.click(screen.getByRole('button', { name: 'Return to dashboard' }));
      expect(onBack).toHaveBeenCalledTimes(1);
    } finally {
      consoleError.mockRestore();
    }
  });
});
