import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { isEditableTarget, SHORTCUT_REFERENCE, useAppShortcuts } from './appShortcuts';

function setup(enabled = true) {
  const onOpenSearch = vi.fn();
  const onOpenShortcuts = vi.fn();
  const onOpenExam = vi.fn();
  const hook = renderHook(() =>
    useAppShortcuts({ onOpenSearch, onOpenShortcuts, onOpenExam }, enabled),
  );
  return { onOpenSearch, onOpenShortcuts, onOpenExam, hook };
}

function press(key: string, options: KeyboardEventInit = {}, target: EventTarget = document.body) {
  const event = new KeyboardEvent('keydown', {
    bubbles: true,
    cancelable: true,
    key,
    ...options,
  });
  target.dispatchEvent(event);
  return event;
}

function editableInput() {
  const input = document.createElement('input');
  document.body.appendChild(input);
  input.focus();
  return input;
}

describe('appShortcuts', () => {
  it('publishes a discoverable reference including the typing exception', () => {
    expect(SHORTCUT_REFERENCE.some((entry) => entry.keys.includes('K'))).toBe(true);
    expect(SHORTCUT_REFERENCE.some((entry) => entry.label.includes('while typing'))).toBe(true);
    expect(SHORTCUT_REFERENCE).toHaveLength(9);
  });

  it('detects editable targets', () => {
    const input = document.createElement('input');
    const textarea = document.createElement('textarea');
    const select = document.createElement('select');
    const editable = document.createElement('div');
    editable.setAttribute('contenteditable', 'true');
    for (const element of [input, textarea, select, editable]) {
      expect(isEditableTarget(element)).toBe(true);
    }
    expect(isEditableTarget(document.body)).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });

  it('opens search with Ctrl+K, including while typing in an input', () => {
    const { onOpenSearch } = setup();
    const bodyEvent = press('k', { ctrlKey: true });
    expect(onOpenSearch).toHaveBeenCalledTimes(1);
    expect(bodyEvent.defaultPrevented).toBe(true);

    const input = editableInput();
    press('k', { ctrlKey: true }, input);
    expect(onOpenSearch).toHaveBeenCalledTimes(2);
    input.remove();
  });

  it('navigates to exam workspaces with Ctrl+1..4 outside inputs', () => {
    const { onOpenExam } = setup();
    for (const [key, code] of [
      ['1', 'csir-net-physics'],
      ['2', 'gate-physics'],
      ['3', 'tifr-gs-physics'],
      ['4', 'jest-physics'],
    ] as const) {
      press(key, { ctrlKey: true });
      expect(onOpenExam).toHaveBeenLastCalledWith(code);
    }
    expect(onOpenExam).toHaveBeenCalledTimes(4);

    const input = editableInput();
    press('2', { ctrlKey: true }, input);
    expect(onOpenExam).toHaveBeenCalledTimes(4);
    input.remove();
  });

  it('ignores shortcuts without Ctrl, with extra modifiers, and key repeats', () => {
    const { onOpenSearch, onOpenExam, onOpenShortcuts } = setup();
    press('1');
    press('2', { ctrlKey: true, shiftKey: true });
    press('3', { ctrlKey: true, altKey: true });
    press('k', { ctrlKey: true, repeat: true });
    press('k', { ctrlKey: true, shiftKey: true });
    press('/');
    expect(onOpenExam).not.toHaveBeenCalled();
    expect(onOpenSearch).not.toHaveBeenCalled();
    expect(onOpenShortcuts).not.toHaveBeenCalled();
  });

  it('opens the shortcut reference with Ctrl+/ outside inputs only', () => {
    const { onOpenShortcuts } = setup();
    const event = press('/', { ctrlKey: true });
    expect(onOpenShortcuts).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(true);

    const input = editableInput();
    press('/', { ctrlKey: true }, input);
    expect(onOpenShortcuts).toHaveBeenCalledTimes(1);
    input.remove();
  });

  it('disables every shortcut when disabled and cleans its listener up', () => {
    const disabled = setup(false);
    press('k', { ctrlKey: true });
    press('1', { ctrlKey: true });
    expect(disabled.onOpenSearch).not.toHaveBeenCalled();
    expect(disabled.onOpenExam).not.toHaveBeenCalled();
    disabled.hook.unmount();

    const enabled = setup(true);
    press('k', { ctrlKey: true });
    expect(enabled.onOpenSearch).toHaveBeenCalledTimes(1);
    enabled.hook.unmount();
    press('k', { ctrlKey: true });
    expect(enabled.onOpenSearch).toHaveBeenCalledTimes(1);
  });
});
