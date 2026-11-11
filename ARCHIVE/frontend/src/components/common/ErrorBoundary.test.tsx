import { render, screen, fireEvent } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorBoundary } from './ErrorBoundary';

const ThrowingChild = ({ shouldThrow }: { shouldThrow: boolean }) => {
  if (shouldThrow) throw new Error('chart render failed');
  return <div>Recovered view</div>;
};

describe('ErrorBoundary', () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it('renders a recovery panel when a child crashes', () => {
    render(
      <ErrorBoundary>
        <ThrowingChild shouldThrow />
      </ErrorBoundary>
    );

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('chart render failed')).toBeInTheDocument();
  });

  it('can retry rendering after an error', () => {
    const { rerender } = render(
      <ErrorBoundary>
        <ThrowingChild shouldThrow />
      </ErrorBoundary>
    );

    rerender(
      <ErrorBoundary>
        <ThrowingChild shouldThrow={false} />
      </ErrorBoundary>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(screen.getByText('Recovered view')).toBeInTheDocument();
  });
});
