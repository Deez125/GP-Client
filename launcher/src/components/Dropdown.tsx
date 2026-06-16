import { useEffect, useRef, useState } from "react";
import { LuChevronDown } from "react-icons/lu";

export interface DropdownOption {
  value: string;
  label: string;
}

// Themed dropdown that replaces the native <select> (whose open option list
// can't be styled in Chromium/WebView2 on Windows).
export function Dropdown({
  value,
  options,
  onChange,
  disabled,
  openUp,
  placeholder,
}: {
  value: string;
  options: DropdownOption[];
  onChange: (v: string) => void;
  disabled?: boolean;
  /** Open the menu upward (e.g. when the trigger sits at the bottom). */
  openUp?: boolean;
  placeholder?: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const current = options.find((o) => o.value === value);

  return (
    <div className={`dropdown${disabled ? " disabled" : ""}`} ref={ref}>
      <button
        type="button"
        className="dropdown-trigger"
        onClick={() => !disabled && setOpen((o) => !o)}
        disabled={disabled}
      >
        <span className="dropdown-value">{current?.label ?? placeholder ?? ""}</span>
        <LuChevronDown className={`dropdown-arrow${open ? " open" : ""}`} />
      </button>

      {open && (
        <ul className={`dropdown-menu${openUp ? " up" : ""}`}>
          {options.map((o) => (
            <li
              key={o.value}
              className={`dropdown-option${o.value === value ? " selected" : ""}`}
              onClick={() => {
                onChange(o.value);
                setOpen(false);
              }}
            >
              {o.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
