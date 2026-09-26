import React from "react";

import { focusReturnTarget } from "../ui/contextMenu/contextMenuStore";

const FOCUSABLE =
  "input:not([disabled]), textarea:not([disabled]), select:not([disabled]), button:not([disabled]):not([tabindex='-1']), a[href], [tabindex]:not([tabindex='-1'])";

export function useDialogFocus(ref: React.RefObject<HTMLElement>, open: boolean, key?: unknown) {
  React.useEffect(() => {
    if (!open) return;
    let previous: HTMLElement | null = null;
    const frame = requestAnimationFrame(() => {
      previous = focusReturnTarget();
      const dialog = ref.current;
      if (!dialog) return;
      const first = dialog.querySelector<HTMLElement>(FOCUSABLE);
      if (first) {
        first.focus();
        return;
      }
      if (!dialog.hasAttribute("tabindex")) dialog.setAttribute("tabindex", "-1");
      dialog.focus();
    });
    return () => {
      cancelAnimationFrame(frame);
      if (previous && previous.isConnected) previous.focus({ preventScroll: true });
    };
  }, [ref, open, key]);
}
