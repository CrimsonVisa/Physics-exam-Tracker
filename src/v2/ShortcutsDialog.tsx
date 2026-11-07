import { SHORTCUT_REFERENCE } from '../lib/appShortcuts';
import { DialogShell } from './DialogShell';

export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  return (
    <DialogShell ariaLabel="Keyboard shortcuts" className="v2-shortcuts-dialog" onClose={onClose}>
      <header className="v2-shortcuts-head">
        <div>
          <p className="eyebrow">Keyboard</p>
          <h2>Keyboard shortcuts</h2>
        </div>
        <button
          aria-label="Close keyboard shortcuts"
          className="secondary-button v2-dialog-close"
          data-autofocus
          onClick={onClose}
          type="button"
        >
          Close
        </button>
      </header>
      <ul className="v2-shortcut-list">
        {SHORTCUT_REFERENCE.map((entry) => (
          <li key={`${entry.keys.join('-')}-${entry.label}`}>
            <span className="v2-shortcut-keys">
              {entry.keys.map((key) => (
                <kbd key={key}>{key}</kbd>
              ))}
            </span>
            <span className="v2-shortcut-label">{entry.label}</span>
          </li>
        ))}
      </ul>
      <p className="v2-shortcuts-note">
        Navigation shortcuts pause while a text field is focused; Ctrl+K always works. Timer and
        destructive actions intentionally have no keyboard shortcuts.
      </p>
    </DialogShell>
  );
}
