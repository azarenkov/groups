import React from "react";

const fit = (el: HTMLTextAreaElement) => {
  el.style.height = "auto";
  el.style.height = `${el.scrollHeight}px`;
};

export function useAutoGrow(
  ref: React.RefObject<HTMLTextAreaElement>,
  value: string,
): void {
  const watched = React.useRef<{ el: HTMLTextAreaElement; observer: ResizeObserver } | null>(null);

  React.useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    fit(el);
  }, [ref, value]);

  React.useEffect(() => {
    const el = ref.current;
    if (watched.current?.el === el) return;
    watched.current?.observer.disconnect();
    watched.current = null;
    if (!el || typeof ResizeObserver === "undefined") return;
    let width = el.clientWidth;
    const observer = new ResizeObserver(() => {
      if (el.clientWidth === width) return;
      width = el.clientWidth;
      fit(el);
    });
    observer.observe(el);
    watched.current = { el, observer };
  });

  React.useEffect(
    () => () => {
      watched.current?.observer.disconnect();
      watched.current = null;
    },
    [],
  );
}
