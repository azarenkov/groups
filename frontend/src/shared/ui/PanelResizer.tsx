import React from "react";

import { clampPanelWidth } from "../lib/panelWidth";

interface PanelResizerProps {
  cssVar: string;
  otherSelector: string;
  side: "start" | "end";
  min: number;
  max: number;
  fallback: number;
  storageKey: string;
  label: string;
}

const KEY_STEP = 16;
const KEY_STEP_LARGE = 48;

const readVar = (name: string, fallback: number): number => {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name);
  const value = parseFloat(raw);
  return Number.isFinite(value) ? value : fallback;
};

const readStored = (key: string): number | null => {
  try {
    const value = Number(window.localStorage.getItem(key));
    return Number.isFinite(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
};

const store = (key: string, value: number | null) => {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, String(value));
  } catch {}
};

const PanelResizer: React.FC<PanelResizerProps> = ({
  cssVar,
  otherSelector,
  side,
  min,
  max,
  fallback,
  storageKey,
  label,
}) => {
  const [width, setWidth] = React.useState(fallback);
  const [dragging, setDragging] = React.useState(false);
  const dragRef = React.useRef<{ x: number; width: number } | null>(null);
  const preferredRef = React.useRef<number | null>(null);

  const apply = React.useCallback(
    (requested: number) => {
      const next = clampPanelWidth(
        requested,
        { min, max },
        window.innerWidth,
        document.querySelector(otherSelector)?.getBoundingClientRect().width ?? 0,
      );
      document.documentElement.style.setProperty(cssVar, `${next}px`);
      setWidth(next);
      return next;
    },
    [cssVar, otherSelector, min, max],
  );

  React.useEffect(() => {
    const stored = readStored(storageKey);
    preferredRef.current = stored;
    if (stored !== null) apply(stored);
    else setWidth(readVar(cssVar, fallback));
  }, [apply, storageKey, cssVar, fallback]);

  React.useEffect(() => {
    const onResize = () => {
      if (preferredRef.current !== null) apply(preferredRef.current);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [apply]);

  const remember = (value: number) => {
    preferredRef.current = value;
    store(storageKey, value);
  };

  const reset = () => {
    document.documentElement.style.removeProperty(cssVar);
    preferredRef.current = null;
    store(storageKey, null);
    setWidth(readVar(cssVar, fallback));
  };

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    dragRef.current = { x: e.clientX, width: readVar(cssVar, fallback) };
    setDragging(true);
  };

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag) return;
    const delta = e.clientX - drag.x;
    apply(side === "start" ? drag.width + delta : drag.width - delta);
  };

  const finish = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current) return;
    dragRef.current = null;
    setDragging(false);
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
    remember(readVar(cssVar, fallback));
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    const step = e.shiftKey ? KEY_STEP_LARGE : KEY_STEP;
    const grow = (e.key === "ArrowRight") === (side === "start");
    remember(apply(width + (grow ? step : -step)));
  };

  return (
    <div
      className={"panel-resizer " + side + (dragging ? " dragging" : "")}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={width}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={finish}
      onPointerCancel={finish}
      onDoubleClick={reset}
      onKeyDown={onKeyDown}
    />
  );
};

export default PanelResizer;
