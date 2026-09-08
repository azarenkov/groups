import React from "react";

import { useHotkeySettingsStore } from "../store/hotkeySettingsStore";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";
import {
  DESKTOP_SHORTCUTS,
  HOTKEY_ACTIONS,
  bindingAvailable,
  bindingIsBareKey,
  eventMatches,
  hasStrongModifier,
  hotkeyAction,
  isTextEntryTarget,
  isValidBinding,
  keyTokenFromCode,
  keyTokenFromEvent,
  parseBinding,
  type HotkeyAction,
  type HotkeyActionId,
} from "../utils/hotkeys";

export type BindingBlocker =
  | { kind: "platform" }
  | { kind: "desktop"; id: string }
  | { kind: "pushToTalk" }
  | null;

export function bindingBlocker(action: HotkeyAction, binding: string): BindingBlocker {
  if (!bindingAvailable(binding)) return { kind: "platform" };
  if (typeof window !== "undefined" && window.electron) {
    const reserved = DESKTOP_SHORTCUTS.find((d) => d.binding === binding);
    if (reserved) return { kind: "desktop", id: reserved.id };
  }
  if (
    action.scope === "global" &&
    bindingIsBareKey(binding, keyTokenFromCode(useVoiceSettingsStore.getState().pttKey))
  ) {
    return { kind: "pushToTalk" };
  }
  return null;
}

type HotkeyHandler = () => void;

const handlers = new Map<HotkeyActionId, HotkeyHandler[]>();
let installed = false;

export function bindingFor(id: HotkeyActionId): string {
  const override = useHotkeySettingsStore.getState().bindings[id];
  return isValidBinding(override) ? override : hotkeyAction(id).defaultBinding;
}

export function registerHotkey(id: HotkeyActionId, handler: HotkeyHandler): () => void {
  const stack = handlers.get(id) ?? [];
  stack.push(handler);
  handlers.set(id, stack);
  return () => {
    const current = handlers.get(id);
    if (!current) return;
    const index = current.lastIndexOf(handler);
    if (index >= 0) current.splice(index, 1);
  };
}

export function useHotkey(id: HotkeyActionId, handler: HotkeyHandler, enabled = true): void {
  const ref = React.useRef(handler);
  ref.current = handler;
  React.useEffect(() => {
    if (!enabled) return;
    return registerHotkey(id, () => ref.current());
  }, [id, enabled]);
}

export function initHotkeys(): void {
  if (installed) return;
  installed = true;
  window.addEventListener("keydown", (e) => {
    if (e.defaultPrevented || e.isComposing || e.repeat) return;
    if (!keyTokenFromEvent(e)) return;
    for (const action of HOTKEY_ACTIONS) {
      if (action.scope !== "global") continue;
      const stack = handlers.get(action.id);
      if (!stack || stack.length === 0) continue;
      const binding = bindingFor(action.id);
      if (!eventMatches(e, binding)) continue;
      if (bindingBlocker(action, binding) !== null) return;
      const parsed = parseBinding(binding);
      if (parsed && !hasStrongModifier(parsed) && isTextEntryTarget(e.target)) return;
      e.preventDefault();
      stack[stack.length - 1]();
      return;
    }
  });
}
