import React from "react";

export function useAutoGrow(
  ref: React.RefObject<HTMLTextAreaElement>,
  value: string,
): void {
  React.useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [ref, value]);
}
