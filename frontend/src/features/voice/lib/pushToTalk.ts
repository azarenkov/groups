import { getLiveKitService } from "./livekit";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";

const MODIFIER_KEYS = ["metaKey", "ctrlKey", "altKey", "shiftKey"] as const;
const MODIFIER_FLAGS: Record<string, (typeof MODIFIER_KEYS)[number]> = {
  ShiftLeft: "shiftKey",
  ShiftRight: "shiftKey",
  ControlLeft: "ctrlKey",
  ControlRight: "ctrlKey",
  AltLeft: "altKey",
  AltRight: "altKey",
  MetaLeft: "metaKey",
  MetaRight: "metaKey",
};

let initialized = false;
let holding = false;
let holdingCode: string | null = null;
let globalBound = false;
let globalSeq = 0;

const listeners = new Set<(holding: boolean) => void>();

const isTextEntryTarget = (target: EventTarget | null): boolean => {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  return (
    el.closest('input, textarea, select, [contenteditable="true"]') !== null ||
    el.isContentEditable
  );
};

const notify = () => {
  listeners.forEach((fn) => {
    try {
      fn(holding);
    } catch (err) {
      console.warn("[ptt] listener failed:", err);
    }
  });
};

const startTransmitting = (code: string | null) => {
  if (holding) return;
  holding = true;
  holdingCode = code;
  getLiveKitService().pttSetTransmitting(true);
  notify();
};

const stopTransmitting = () => {
  if (!holding) return;
  holding = false;
  holdingCode = null;
  getLiveKitService().pttSetTransmitting(false);
  notify();
};

const applyEnforcement = (enabled: boolean) => {
  stopTransmitting();
  getLiveKitService().setPushToTalkEnforced(enabled);
};

export const isPushToTalkHeld = (): boolean => holding;

export const subscribePushToTalk = (
  fn: (holding: boolean) => void,
): (() => void) => {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
};

export const pushToTalkGlobalSupported = (): boolean =>
  typeof window !== "undefined" &&
  typeof window.electron?.setGlobalPushToTalk === "function";

const syncGlobalBinding = async () => {
  if (!pushToTalkGlobalSupported()) return;
  const { pushToTalk, pttKey, pttGlobal } = useVoiceSettingsStore.getState();
  const code = pushToTalk && pttGlobal ? pttKey : null;
  const seq = ++globalSeq;
  let bound = false;
  try {
    bound = (await window.electron!.setGlobalPushToTalk!(code)) === true;
  } catch (err) {
    console.warn("[ptt] the global hotkey could not be registered:", err);
  }
  if (seq !== globalSeq) return;
  globalBound = bound;
  if (!bound) stopTransmitting();
};

export const initPushToTalk = (): void => {
  if (initialized) return;
  initialized = true;
  applyEnforcement(useVoiceSettingsStore.getState().pushToTalk);
  void syncGlobalBinding();
  useVoiceSettingsStore.subscribe((state, prev) => {
    if (state.pushToTalk !== prev.pushToTalk) {
      applyEnforcement(state.pushToTalk);
      void syncGlobalBinding();
      return;
    }
    if (state.pttKey !== prev.pttKey || state.pttGlobal !== prev.pttGlobal) {
      stopTransmitting();
      void syncGlobalBinding();
    }
  });
  window.electron?.onPushToTalk?.((down) => {
    if (!globalBound) return;
    if (!useVoiceSettingsStore.getState().pushToTalk) return;
    if (!down) {
      stopTransmitting();
      return;
    }
    if (document.hasFocus() && isTextEntryTarget(document.activeElement)) return;
    startTransmitting(useVoiceSettingsStore.getState().pttKey);
  });
  window.addEventListener("keydown", (e) => {
    const { pushToTalk, pttKey } = useVoiceSettingsStore.getState();
    if (e.code !== pttKey) return;
    if (!pushToTalk) return;
    const own = MODIFIER_FLAGS[pttKey];
    if (MODIFIER_KEYS.some((flag) => flag !== own && e[flag])) return;
    if (isTextEntryTarget(e.target)) return;
    e.preventDefault();
    startTransmitting(e.code);
  });
  window.addEventListener("keyup", (e) => {
    if (!holding || e.code !== holdingCode) return;
    stopTransmitting();
  });
  window.addEventListener("blur", () => {
    if (globalBound) return;
    stopTransmitting();
  });
};
