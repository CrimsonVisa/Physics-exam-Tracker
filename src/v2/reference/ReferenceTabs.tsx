import type { KeyboardEvent, ReactNode } from 'react';

export type ReferenceTabOption<T extends string> = {
  id: T;
  label: string;
  count?: number;
};

type ReferenceTabsProps<T extends string> = {
  ariaLabel: string;
  active: T;
  options: ReadonlyArray<ReferenceTabOption<T>>;
  onChange: (tab: T) => void;
};

export function ReferenceTabs<T extends string>({
  ariaLabel,
  active,
  options,
  onChange,
}: ReferenceTabsProps<T>) {
  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let nextIndex: number | null = null;
    if (event.key === 'ArrowRight') {
      nextIndex = (index + 1) % options.length;
    } else if (event.key === 'ArrowLeft') {
      nextIndex = (index - 1 + options.length) % options.length;
    } else if (event.key === 'Home') {
      nextIndex = 0;
    } else if (event.key === 'End') {
      nextIndex = options.length - 1;
    }
    if (nextIndex === null) {
      return;
    }
    event.preventDefault();
    const next = options[nextIndex];
    onChange(next.id);
    document.getElementById(`reference-tab-${next.id}`)?.focus();
  }

  return (
    <div aria-label={ariaLabel} className="v2-reference-tabs" role="tablist">
      {options.map((option, index) => {
        const selected = option.id === active;
        return (
          <button
            aria-controls={`reference-panel-${option.id}`}
            aria-selected={selected}
            className="v2-reference-tab"
            data-active={selected ? 'true' : undefined}
            id={`reference-tab-${option.id}`}
            key={option.id}
            onClick={() => onChange(option.id)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            role="tab"
            tabIndex={selected ? 0 : -1}
            type="button"
          >
            <span>{option.label}</span>
            {typeof option.count === 'number' ? (
              <span className="v2-reference-tab-count">{option.count}</span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

type ReferencePanelProps = {
  tab: string;
  children: ReactNode;
};

export function ReferencePanel({ tab, children }: ReferencePanelProps) {
  return (
    <div
      aria-labelledby={`reference-tab-${tab}`}
      className="v2-reference-panel"
      id={`reference-panel-${tab}`}
      role="tabpanel"
      tabIndex={0}
    >
      {children}
    </div>
  );
}
