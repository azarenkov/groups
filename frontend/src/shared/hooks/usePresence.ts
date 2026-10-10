import React from "react";

export type PresenceState = "enter" | "shown" | "exit";

const prefersMotion = () =>
  typeof window.matchMedia === "function" && !window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export const usePresence = (open: boolean, durationMs: number): PresenceState | null => {
  const [state, setState] = React.useState<PresenceState | null>(open ? "shown" : null);
  const firstRef = React.useRef(true);

  React.useEffect(() => {
    if (firstRef.current) {
      firstRef.current = false;
      return;
    }
    const animate = prefersMotion();
    if (open) {
      if (!animate) {
        setState("shown");
        return;
      }
      setState("enter");
      const timer = setTimeout(() => setState("shown"), durationMs);
      return () => clearTimeout(timer);
    }
    if (!animate) {
      setState(null);
      return;
    }
    setState((s) => (s === null ? null : "exit"));
    const timer = setTimeout(() => setState(null), durationMs);
    return () => clearTimeout(timer);
  }, [open, durationMs]);

  return state;
};
