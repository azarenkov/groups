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

const PANELS_CHANGED = "rift:panels-changed";

const announcePanels = (source: string) =>
  window.dispatchEvent(new CustomEvent<string>(PANELS_CHANGED, { detail: source }));

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

  const clampFor = React.useCallback(
    (requested: number) =>
      clampPanelWidth(
        requested,
        { min, max },
        window.innerWidth,
        document.querySelector(otherSelector)?.getBoundingClientRect().width ?? 0,
      ),
    [otherSelector, min, max],
  );

  const commit = React.useCallback(
    (next: number, inline: boolean) => {
      const root = document.documentElement.style;
      const before = root.getPropertyValue(cssVar);
      if (inline) root.setProperty(cssVar, `${next}px`);
      else root.removeProperty(cssVar);
      setWidth(next);
      if (root.getPropertyValue(cssVar) !== before) announcePanels(cssVar);
      return next;
    },
    [cssVar],
  );

  const apply = React.useCallback(
    (requested: number) => commit(clampFor(requested), true),
    [clampFor, commit],
  );

  const reclamp = React.useCallback(() => {
    const preferred = preferredRef.current;
    if (preferred !== null) {
      apply(preferred);
      return;
    }
    const root = document.documentElement.style;
    const inline = root.getPropertyValue(cssVar);
    root.removeProperty(cssVar);
    const natural = readVar(cssVar, fallback);
    if (inline) root.setProperty(cssVar, inline);
    const next = clampFor(natural);
    commit(next, next !== natural);
  }, [apply, clampFor, commit, cssVar, fallback]);

  React.useEffect(() => {
    preferredRef.current = readStored(storageKey);
    reclamp();
  }, [reclamp, storageKey]);

  React.useEffect(() => {
    const onPanels = (e: Event) => {
      if ((e as CustomEvent<string>).detail !== cssVar) reclamp();
    };
    window.addEventListener("resize", reclamp);
    window.addEventListener(PANELS_CHANGED, onPanels);
    announcePanels(cssVar);
    return () => {
      window.removeEventListener("resize", reclamp);
      window.removeEventListener(PANELS_CHANGED, onPanels);
      requestAnimationFrame(() => announcePanels(cssVar));
    };
  }, [reclamp, cssVar]);

  const remember = (value: number) => {
    preferredRef.current = value;
    store(storageKey, value);
  };

  const reset = () => {
    preferredRef.current = null;
    store(storageKey, null);
    document.documentElement.style.removeProperty(cssVar);
    reclamp();
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
