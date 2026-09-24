import type React from "react";
import { create } from "zustand";

import type { IconName } from "../Icon";

export type MenuEntry =
  | {
      kind: "item";
      id: string;
      label: string;
      icon?: IconName;
      danger?: boolean;
      disabled?: boolean;
      onSelect: () => void;
    }
  | { kind: "separator"; id: string }
  | { kind: "custom"; id: string; render: () => React.ReactNode };

interface OpenMenu {
  x: number;
  y: number;
  entries: MenuEntry[];
  seq: number;
}

interface ContextMenuState {
  menu: OpenMenu | null;
  open: (x: number, y: number, entries: MenuEntry[]) => void;
  close: () => void;
}

let seq = 0;

export const useContextMenuStore = create<ContextMenuState>()((set) => ({
  menu: null,
  open: (x, y, entries) => {
    seq += 1;
    set({ menu: { x, y, entries, seq } });
  },
  close: () => set({ menu: null }),
}));

export function visibleEntries(entries: MenuEntry[]): MenuEntry[] {
  const out: MenuEntry[] = [];
  for (const entry of entries) {
    if (entry.kind === "separator") {
      if (out.length === 0 || out[out.length - 1].kind === "separator") continue;
    }
    out.push(entry);
  }
  while (out.length > 0 && out[out.length - 1].kind === "separator") out.pop();
  return out;
}

export function openContextMenu(
  e: { clientX: number; clientY: number; preventDefault?: () => void },
  entries: MenuEntry[],
): void {
  const shown = visibleEntries(entries);
  if (shown.length === 0) return;
  e.preventDefault?.();
  useContextMenuStore.getState().open(e.clientX, e.clientY, shown);
}

export function openContextMenuAt(el: Element, entries: MenuEntry[]): void {
  const rect = el.getBoundingClientRect();
  openContextMenu({ clientX: rect.left + 12, clientY: rect.top + rect.height / 2 }, entries);
}
