import { getLiveKitService } from "./livekit";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";

const PTT_CODE = "Space";

let initialized = false;
let holding = false;

const isInteractiveTarget = (target: EventTarget | null): boolean => {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  return (
    el.closest(
      'input, textarea, select, button, a[href], [contenteditable="true"], [role="button"], [role="menuitem"], [role="tab"], [role="option"], [role="switch"], [role="slider"]',
    ) !== null || el.isContentEditable
  );
};

const applyEnforcement = (enabled: boolean) => {
  holding = false;
  getLiveKitService().setPushToTalkEnforced(enabled);
};

export const initPushToTalk = (): void => {
  if (initialized) return;
  initialized = true;
  applyEnforcement(useVoiceSettingsStore.getState().pushToTalk);
  useVoiceSettingsStore.subscribe((state, prev) => {
    if (state.pushToTalk !== prev.pushToTalk) {
      applyEnforcement(state.pushToTalk);
    }
  });
  window.addEventListener("keydown", (e) => {
    if (e.code !== PTT_CODE) return;
    if (!useVoiceSettingsStore.getState().pushToTalk) return;
    if (isInteractiveTarget(e.target)) return;
    e.preventDefault();
    if (holding) return;
    holding = true;
    getLiveKitService().pttSetTransmitting(true);
  });
  window.addEventListener("keyup", (e) => {
    if (e.code !== PTT_CODE) return;
    if (!holding) return;
    holding = false;
    if (!useVoiceSettingsStore.getState().pushToTalk) return;
    getLiveKitService().pttSetTransmitting(false);
  });
  window.addEventListener("blur", () => {
    if (!holding) return;
    holding = false;
    if (useVoiceSettingsStore.getState().pushToTalk) {
      getLiveKitService().pttSetTransmitting(false);
    }
  });
};
