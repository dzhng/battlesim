import { useState, useSyncExternalStore, type KeyboardEvent, type ReactNode } from "react";
import { spoken } from "../battleLinks";

/** A value picked from a short list that opens under it, closing on a pick,
 *  on Escape or when focus leaves it. */
export function Pick<T extends string>({
  label,
  testId,
  className,
  options,
  value,
  onChange,
  say = spoken,
  children,
  disabled = false,
}: {
  label: string;
  testId: string;
  className: string;
  options: readonly T[];
  value: T;
  onChange: (value: T) => void;
  say?: (option: T) => string;
  children: ReactNode;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const close = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || !open) return;
    // Escape closes the list, not the page.
    event.stopPropagation();
    setOpen(false);
  };
  return (
    <span
      className={`lobby-pick ${className}`}
      data-open={open || undefined}
      data-readonly={disabled || undefined}
      onKeyDown={close}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
    >
      <button
        type="button"
        disabled={disabled}
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        data-testid={testId}
        onClick={() => setOpen(!open)}
      >
        {children}
      </button>
      {open && (
        <ul role="listbox" aria-label={label}>
          {options.map((option) => (
            <li key={option}>
              <button
                type="button"
                role="option"
                aria-selected={option === value}
                data-testid={`${testId}-${option}`}
                onClick={() => {
                  setOpen(false);
                  if (option !== value) onChange(option);
                }}
              >
                {say(option)}
              </button>
            </li>
          ))}
        </ul>
      )}
    </span>
  );
}

/** Windows narrower than this fold the strip's longest choices into
 *  pickers, so the strip stays one row down to the smallest window the game
 *  supports (1280 wide: web/README.md). */
const FOLDED_STRIP = "(max-width: 1499px)";
const stripFolds = () => globalThis.matchMedia?.(FOLDED_STRIP).matches ?? false;
const watchFold = (change: () => void) => {
  const query = globalThis.matchMedia?.(FOLDED_STRIP);
  query?.addEventListener("change", change);
  return () => query?.removeEventListener("change", change);
};

/** One of a few named options, side by side. An option can be closed. One
 *  that `folds` is a picker of its options in a narrow window. */
export function Choice<T extends string>({
  label,
  options,
  value,
  onChange,
  closed = () => false,
  say = spoken,
  folds = false,
  disabled = false,
}: {
  label: string;
  options: readonly T[];
  value: T | null;
  onChange: (value: T) => void;
  closed?: (option: T) => boolean;
  say?: (option: T) => string;
  folds?: boolean;
  disabled?: boolean;
}) {
  const folded = useSyncExternalStore(watchFold, stripFolds) && folds && value !== null;
  return (
    <div className="lobby-choice">
      <span className="lobby-label" id={`menu-${label}`}>
        {label}
      </span>
      {folded ? (
        <Pick
          disabled={disabled}
          label={label}
          testId={`menu-${label}`}
          className="lobby-folded"
          options={options.filter((option) => !closed(option))}
          value={value}
          onChange={onChange}
          say={say}
        >
          {say(value)}
        </Pick>
      ) : (
        <div role="radiogroup" aria-labelledby={`menu-${label}`} className="lobby-options">
          {options.map((option) => (
            <button
              key={option}
              type="button"
              role="radio"
              aria-checked={option === value}
              disabled={disabled || closed(option)}
              data-readonly={disabled || undefined}
              data-testid={`menu-${label}-${option}`}
              onClick={() => option !== value && onChange(option)}
            >
              {say(option)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
