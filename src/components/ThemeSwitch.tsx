import type { Theme } from '../lib/uiPreferences';
import { Icon } from './NavIcon';

const THEME_OPTIONS = [
  { value: 'dark' as const, label: 'Dark theme', shortLabel: 'Dark', icon: 'moon' as const },
  { value: 'light' as const, label: 'Light theme', shortLabel: 'Light', icon: 'sun' as const },
];

export function ThemeSwitch({
  theme,
  onChange,
  compact = false,
}: {
  theme: Theme;
  onChange: (theme: Theme) => void;
  compact?: boolean;
}) {
  return (
    <div
      aria-label="Color theme"
      className={`theme-switch${compact ? ' theme-switch-compact' : ''}`}
      role="group"
    >
      {THEME_OPTIONS.map((option) => (
        <button
          aria-label={option.label}
          aria-pressed={theme === option.value}
          className={`theme-option${theme === option.value ? ' theme-option-active' : ''}`}
          key={option.value}
          onClick={() => onChange(option.value)}
          title={option.label}
          type="button"
        >
          <Icon name={option.icon} size={15} />
          <span>{option.shortLabel}</span>
          <Icon className="theme-option-check" name="check" size={13} />
        </button>
      ))}
    </div>
  );
}
