import React from "react";

import { useAuthStore } from "../../auth/store/authStore";
import { useHotkeySettingsStore, type HotkeyBindings } from "../store/hotkeySettingsStore";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import {
  HOTKEY_ACTIONS,
  bindingAvailable,
  bindingIsBareKey,
  desktopReservedId,
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
} from "./hotkeyMatching";

export type BindingBlocker =
  | { kind: "platform" }
  | { kind: "desktop"; id: string }
  | { kind: "pushToTalk" }
  | null;

export function pushToTalkReserves(action: HotkeyAction, binding: string): boolean {
  const { pushToTalk, pttKey } = useVoiceSettingsStore.getState();
  return (
    pushToTalk &&
    action.scope === "global" &&
    bindingIsBareKey(binding, keyTokenFromCode(pttKey))
  );
}

export function bindingBlocker(action: HotkeyAction, binding: string): BindingBlocker {
  if (!bindingAvailable(binding)) return { kind: "platform" };
  if (typeof window !== "undefined" && window.electron) {
    const reserved = desktopReservedId(binding);
    if (reserved) return { kind: "desktop", id: reserved };
  }
  if (pushToTalkReserves(action, binding)) return { kind: "pushToTalk" };
  return null;
}

type HotkeyHandler = () => void;

const handlers = new Map<HotkeyActionId, HotkeyHandler[]>();
let installed = false;

const EMPTY_BINDINGS: HotkeyBindings = {};

export function activeBindings(): HotkeyBindings {
  const userId = useAuthStore.getState().user?.id ?? null;
  const { bindings, ownerId } = useHotkeySettingsStore.getState();
  return userId !== null && ownerId === userId ? bindings : EMPTY_BINDINGS;
}

export function useActiveBindings(): HotkeyBindings {
  const userId = useAuthStore((s) => s.user?.id ?? null);
  return useHotkeySettingsStore((s) =>
    userId !== null && s.ownerId === userId ? s.bindings : EMPTY_BINDINGS,
  );
}

export function defaultClaimedBy(
  bindings: HotkeyBindings,
  id: HotkeyActionId,
): HotkeyActionId | null {
  const fallback = hotkeyAction(id).defaultBinding;
  const claimer = HOTKEY_ACTIONS.find(
    (a) => a.id !== id && isValidBinding(bindings[a.id]) && bindings[a.id] === fallback,
  );
  return claimer ? claimer.id : null;
}

export function effectiveBinding(
  bindings: HotkeyBindings,
  id: HotkeyActionId,
): string | null {
  const override = bindings[id];
  if (isValidBinding(override)) return override;
  return defaultClaimedBy(bindings, id) === null ? hotkeyAction(id).defaultBinding : null;
}

export function bindingFor(id: HotkeyActionId): string | null {
  return effectiveBinding(activeBindings(), id);
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
      if (binding === null || !eventMatches(e, binding)) continue;
      if (bindingBlocker(action, binding) !== null) return;
      const parsed = parseBinding(binding);
      if (parsed && !hasStrongModifier(parsed) && isTextEntryTarget(e.target)) return;
      e.preventDefault();
      stack[stack.length - 1]();
      return;
    }
  });
}
