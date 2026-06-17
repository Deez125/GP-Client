import { useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

// Themed tooltip. Renders through a portal positioned above the wrapped element
// so it never gets clipped by an overflow:hidden ancestor (e.g. the account
// box). Use around icon-only buttons that have no visible label.
export function Tooltip({
  text,
  children,
  disabled,
}: {
  text: string;
  children: ReactNode;
  /** Suppress the tooltip (e.g. when the wrapped control is disabled). */
  disabled?: boolean;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);

  return (
    <span
      ref={ref}
      className="tip-wrap"
      onMouseEnter={() => {
        if (disabled) return;
        const r = ref.current?.getBoundingClientRect();
        if (r) setPos({ x: r.left + r.width / 2, y: r.top });
      }}
      onMouseLeave={() => setPos(null)}
    >
      {children}
      {pos &&
        createPortal(
          <div className="tip" style={{ left: pos.x, top: pos.y }}>
            {text}
          </div>,
          document.body,
        )}
    </span>
  );
}
