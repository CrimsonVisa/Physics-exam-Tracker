import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DialogShell } from './DialogShell';

function Harness({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <>
      <button type="button">Open trigger</button>
      {open && (
        <DialogShell ariaLabel="Test dialog" onClose={onClose}>
          <button data-autofocus type="button">
            First action
          </button>
          <button type="button">Last action</button>
        </DialogShell>
      )}
    </>
  );
}

describe('DialogShell', () => {
  it('renders an accessible modal and focuses the autofocus element', async () => {
    render(<Harness onClose={vi.fn()} open />);
    expect(screen.getByRole('dialog', { name: 'Test dialog' })).toHaveAttribute(
      'aria-modal',
      'true',
    );
    await waitFor(() => expect(screen.getByRole('button', { name: 'First action' })).toHaveFocus());
  });

  it('traps Tab and Shift+Tab inside the dialog', async () => {
    const user = userEvent.setup();
    render(<Harness onClose={vi.fn()} open />);
    const first = screen.getByRole('button', { name: 'First action' });
    const last = screen.getByRole('button', { name: 'Last action' });
    await waitFor(() => expect(first).toHaveFocus());

    await user.tab({ shift: true });
    expect(last).toHaveFocus();

    await user.tab();
    expect(first).toHaveFocus();
  });

  it('closes on Escape and restores focus to the opener', async () => {
    const onClose = vi.fn();
    const { rerender } = render(<Harness onClose={onClose} open={false} />);
    const trigger = screen.getByRole('button', { name: 'Open trigger' });
    trigger.focus();

    rerender(<Harness onClose={onClose} open />);
    await waitFor(() => expect(screen.getByRole('button', { name: 'First action' })).toHaveFocus());

    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledTimes(1);

    rerender(<Harness onClose={onClose} open={false} />);
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it('closes when the backdrop is clicked', () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} open />);
    const backdrop = document.querySelector('.v2-dialog-backdrop');
    expect(backdrop).not.toBeNull();
    fireEvent.mouseDown(backdrop as Element);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
